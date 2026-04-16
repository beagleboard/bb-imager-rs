use tokio::sync::mpsc;

use crate::Status;

const ALIGNMENT: usize = 8;

pub(crate) struct Firmware {
    pub(crate) file: bin_file::BinFile,
    pub(crate) crc: zerocopy::little_endian::U32,
    pub(crate) max_addr: u32,
}

impl Firmware {
    pub(crate) fn parse(data: &[u8]) -> crate::Result<Self> {
        if let Ok(s) = std::str::from_utf8(data)
            && let Ok(t) = s.parse::<bin_file::BinFile>()
        {
            let max_addr = t.maximum_address().ok_or(crate::Error::InvalidImage)?;
            let bin_file = t
                .to_bytes(0..max_addr, Some(0xff))
                .map_err(|_| crate::Error::InvalidImage)?;

            let file = Self::from_binary(&bin_file).map_err(|_| crate::Error::InvalidImage)?;
            let max_addr = file
                .maximum_address()
                .ok_or(crate::Error::InvalidImage)?
                .try_into()
                .unwrap();

            return Ok(Self {
                file,
                crc: crate::bsl::crc(&bin_file),
                max_addr,
            });
        }

        let file = Self::from_binary(data).map_err(|_| crate::Error::InvalidImage)?;
        let max_addr = file
            .maximum_address()
            .ok_or(crate::Error::InvalidImage)?
            .try_into()
            .unwrap();

        Ok(Self {
            file,
            crc: crate::bsl::crc(data),
            max_addr,
        })
    }

    /// Need size and length to be 8 byte aligned.
    fn from_binary(data: &[u8]) -> Result<bin_file::BinFile, bin_file::Error> {
        let mut addr = 0;
        let mut binfile = bin_file::BinFile::new();

        let (chunks, rem) = data.as_chunks::<ALIGNMENT>();
        for c in chunks {
            if c.iter().any(|x| *x != 0xff) {
                binfile.add_bytes(c, Some(addr), false)?;
            }

            addr += c.len();
        }

        // Add rem if any non-empty byte.
        if rem.iter().any(|x| *x != 0xff) {
            let extend_len = rem.len().next_multiple_of(ALIGNMENT) - rem.len();
            let mut data: Vec<u8> = rem.to_vec();

            data.extend(std::iter::repeat_n(0xFF, extend_len));

            binfile.add_bytes(data, Some(addr), false)?;
        }

        Ok(binfile)
    }
}

pub(crate) fn check_token(
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> crate::Result<()> {
    match cancel {
        Some(x) if x.is_cancelled() => Err(crate::Error::Aborted),
        _ => Ok(()),
    }
}

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<Status>>, msg: Status) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub fn flash<P, D>(
    firmware: &[u8],
    port_open: P,
    verify: bool,
    mut chan: Option<mpsc::Sender<Status>>,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> crate::Result<()>
where
    P: FnOnce() -> crate::Result<D>,
    D: std::io::Read + std::io::Write,
{
    let firmware = Firmware::parse(firmware)?;

    chan_send(chan.as_mut(), Status::Preparing);

    let port = port_open()?;
    let mut mspm0 = crate::bsl::Mspm0::new(port)?;
    tracing::info!("MSPM0 Connected");

    mspm0.unlock()?;

    check_token(cancel.as_ref())?;

    chan_send(chan.as_mut(), Status::Flashing(0.0));
    check_token(cancel.as_ref())?;
    mspm0.mass_erase()?;

    tracing::info!("Start Flashing");

    check_token(cancel.as_ref())?;

    for (addr, data) in firmware
        .file
        .chunks(Some(mspm0.program_data_max_len()), Some(8))
        .unwrap()
    {
        chan_send(
            chan.as_mut(),
            Status::Flashing(addr as f32 / firmware.max_addr as f32),
        );
        mspm0.program_data(addr as u32, &data)?;
        tracing::debug!("Cur address: {}", addr);
    }

    chan_send(chan.as_mut(), Status::Flashing(1.0));
    chan_send(chan.as_mut(), Status::Verifying);

    if verify {
        let actual_crc = mspm0.standalone_verification(firmware.max_addr)?;
        let actual = u32::from(actual_crc);
        let expected = u32::from(firmware.crc);

        tracing::info!(
            "Standalone verification CRC expected={expected:#010x} actual={actual:#010x}"
        );

        if actual != expected {
            return Err(crate::Error::VerificationMismatch { expected, actual });
        }
    }

    mspm0.start_application()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALIGNMENT: usize = 8;

    /// Binary with one aligned non-0xff block should produce one segment
    #[test]
    fn from_binary_single_block() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];

        let bin = Firmware::from_binary(&data).unwrap();

        let bytes = bin.to_bytes(0..ALIGNMENT, Some(0xff)).unwrap();

        assert_eq!(bytes, data);
    }

    /// 0xff aligned blocks should be skipped
    #[test]
    fn from_binary_skip_ff_block() {
        let data = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

        let bin = Firmware::from_binary(&data).unwrap();

        let bytes = bin.to_bytes(0..ALIGNMENT, Some(0xff)).unwrap();

        assert_eq!(bytes.len(), 0);
    }

    /// Mixed 0xff and data blocks should only store data blocks
    #[test]
    fn from_binary_mixed_blocks() {
        let data = [
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 1, 2, 3, 4, 5, 6, 7, 8,
        ];

        let bin = Firmware::from_binary(&data).unwrap();

        let bytes = bin.to_bytes(0..16, Some(0xff)).unwrap();

        assert_eq!(bytes, data);
    }

    /// Remainder shorter than alignment should still be added
    #[test]
    fn from_binary_with_remainder() {
        let data = [1, 2, 3, 4, 5];

        let bin = Firmware::from_binary(&data).unwrap();

        let bytes = bin.to_bytes(0..ALIGNMENT, Some(0xff)).unwrap();

        let mut expected = [0xff; ALIGNMENT];
        expected[..5].copy_from_slice(&data);

        assert_eq!(bytes, expected);
    }

    /// Multiple aligned blocks should maintain correct addresses
    #[test]
    fn from_binary_multiple_blocks() {
        let data = [
            1, 2, 3, 4, 5, 6, 7, 8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 9, 10, 11, 12,
            13, 14, 15, 16,
        ];

        let bin = Firmware::from_binary(&data).unwrap();

        let bytes = bin.to_bytes(0..24, Some(0xff)).unwrap();

        assert_eq!(bytes, data);
    }

    #[test]
    fn parse_binary_input() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];

        let fw = Firmware::parse(&data).unwrap();

        let bytes = fw.file.to_bytes(0..8, Some(0xff)).unwrap();

        assert_eq!(bytes, data);
    }
}
