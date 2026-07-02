use std::io::{Read, Seek, Write};

use bb_helper::cancel::CancellationToken;

use crate::helpers::{Eject, check_cancel};
use crate::{ContentType, Result};

pub fn flash<F, I>(img: F, dst: crate::Destination, cancel: Option<CancellationToken>) -> Result<()>
where
    F: FnOnce() -> std::io::Result<I>,
    for<'b> &'b mut I: IntoIterator<Item = (Box<str>, ContentType<'b>)>,
{
    tracing::info!("Opening Destination");

    match dst {
        crate::Destination::File(path) => {
            let sd = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)?;
            common(img, sd, cancel)
        }
        crate::Destination::SdCard(path) => {
            let sd = crate::pal::open(&path)?;
            common(img, sd, cancel)
        }
    }
}

fn common<F, I, S>(img: F, mut sd: S, cancel: Option<CancellationToken>) -> Result<()>
where
    F: FnOnce() -> std::io::Result<I>,
    S: Read + Write + Seek + std::fmt::Debug + Eject,
    for<'b> &'b mut I: IntoIterator<Item = (Box<str>, ContentType<'b>)>,
{
    tracing::info!("Opening Image");
    let mut img = img()?;

    check_cancel(cancel.as_ref())?;

    internal((&mut img).into_iter(), &mut sd, cancel)?;

    tracing::info!("Ejecting SD Card");
    let _ = sd.eject();

    Ok(())
}

fn internal<'a, I, S>(imgs: I, sd: S, cancel: Option<CancellationToken>) -> Result<()>
where
    S: Read + Write + Seek + std::fmt::Debug,
    I: Iterator<Item = (Box<str>, ContentType<'a>)>,
{
    tracing::info!("Starting bootfs update");
    let mut sd = crate::helpers::DeviceWrapper::new(sd)?;
    let customization = crate::Customization {
        partition: crate::ParitionType::Boot,
        content: imgs,
    };

    customization.customize(&mut sd, cancel)?;
    sd.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::io::{self, SeekFrom};

    use fscommon::StreamSlice;
    use mbrman::{CHS, MBR, MBRPartitionEntry};

    use super::*;

    const DISK_SIZE: u64 = 128 * 1024 * 1024; // 128 MiB
    const SECTOR_SIZE: u32 = 512;
    const FIRST_LBA: u32 = 2048;

    #[derive(Debug, Clone)]
    struct MockArchive(Vec<(Box<str>, Option<Vec<u8>>)>);

    impl Default for MockArchive {
        fn default() -> Self {
            Self(vec![
                ("config".into(), None),
                ("config/cmdline.txt".into(), Some(b"console=ttyS0".to_vec())),
            ])
        }
    }

    impl IntoIterator for MockArchive {
        type Item = (Box<str>, ContentType<'static>);
        type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

        fn into_iter(self) -> Self::IntoIter {
            Box::new(
                self.0
                    .iter()
                    .map(|(p, f)| match f {
                        Some(x) => (
                            p.clone(),
                            ContentType::Reader(Box::new(io::Cursor::new(x.clone()))),
                        ),
                        None => (p.clone(), ContentType::Dir),
                    })
                    .collect::<Vec<Self::Item>>()
                    .into_iter(),
            )
        }
    }

    impl<'b> IntoIterator for &'b mut MockArchive {
        type Item = (Box<str>, ContentType<'b>);
        type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'b>;

        fn into_iter(self) -> Self::IntoIter {
            Box::new(
                self.0
                    .iter()
                    .map(|(p, f)| match f {
                        Some(x) => (
                            p.clone(),
                            ContentType::Reader(Box::new(io::Cursor::new(x.clone()))),
                        ),
                        None => (p.clone(), ContentType::Dir),
                    })
                    .collect::<Vec<Self::Item>>()
                    .into_iter(),
            )
        }
    }

    #[derive(Debug)]
    struct MockSd {
        file: tempfile::NamedTempFile,
        fail: CancellationToken,
    }

    impl MockSd {
        fn new() -> Self {
            let mut img = tempfile::NamedTempFile::new().unwrap();

            img.as_file().set_len(DISK_SIZE).unwrap();

            let mut mbr = MBR::new_from(&mut img, SECTOR_SIZE, [0x12, 0x34, 0x56, 0x78]).unwrap();

            let total_sectors = (DISK_SIZE / SECTOR_SIZE as u64) as u32;
            let num_sectors = total_sectors - FIRST_LBA;

            mbr[1] = MBRPartitionEntry {
                boot: 0x80,
                first_chs: CHS::empty(),
                sys: 0x0C, // FAT32 (LBA)
                last_chs: CHS::empty(),
                starting_lba: FIRST_LBA,
                sectors: num_sectors,
            };

            mbr.write_into(&mut img).unwrap();

            let partition_offset = FIRST_LBA as u64 * SECTOR_SIZE as u64;
            let partition_size = num_sectors as u64 * SECTOR_SIZE as u64;

            {
                let mut partition = img.reopen().unwrap();

                partition.seek(SeekFrom::Start(partition_offset)).unwrap();

                let mut partition =
                    StreamSlice::new(partition, partition_offset, partition_size).unwrap();

                fatfs::format_volume(
                    &mut partition,
                    fatfs::FormatVolumeOptions::new()
                        .fat_type(fatfs::FatType::Fat32)
                        .volume_label(*b"BOOT       "),
                )
                .unwrap();
            }

            img.rewind().unwrap();

            Self {
                file: img,
                fail: CancellationToken::default(),
            }
        }
    }

    impl Write for MockSd {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.fail.is_cancelled() {
                Err(io::Error::new(io::ErrorKind::QuotaExceeded, "Fail"))
            } else {
                self.file.write(buf)
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.fail.is_cancelled() {
                Err(io::Error::new(io::ErrorKind::QuotaExceeded, "Fail"))
            } else {
                self.file.flush()
            }
        }
    }

    impl Seek for MockSd {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            if self.fail.is_cancelled() {
                Err(io::Error::new(io::ErrorKind::QuotaExceeded, "Fail"))
            } else {
                self.file.seek(pos)
            }
        }
    }

    impl Read for MockSd {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.fail.is_cancelled() {
                Err(io::Error::new(io::ErrorKind::QuotaExceeded, "Fail"))
            } else {
                self.file.read(buf)
            }
        }
    }

    impl Eject for MockSd {
        fn eject(self) -> io::Result<()> {
            self.file.as_file().sync_all()
        }
    }

    #[test]
    fn basic() {
        let iter = MockArchive::default();
        let mut sd = MockSd::new();

        internal(iter.clone().into_iter(), &mut sd, None).unwrap();
        sd.rewind().unwrap();

        let boot_part = crate::customization::ParitionType::Boot.open(sd).unwrap();
        let root = boot_part.root_dir();

        for (path, f) in iter {
            match f {
                ContentType::Dir => {
                    root.open_dir(&path).unwrap();
                }
                ContentType::Reader(mut read) => {
                    let mut dst = root.open_file(&path).unwrap();
                    let mut expected = Vec::new();
                    let mut actual = Vec::new();

                    read.read_to_end(&mut expected).unwrap();
                    dst.read_to_end(&mut actual).unwrap();

                    assert_eq!(actual, expected);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn basic_file_flash() {
        let iter = MockArchive::default();
        let mut sd = MockSd::new();

        let iter_clone = iter.clone();
        flash(
            move || Ok(iter_clone),
            crate::Destination::File(sd.file.path().into()),
            None,
        )
        .unwrap();
        sd.rewind().unwrap();

        let boot_part = crate::customization::ParitionType::Boot.open(sd).unwrap();
        let root = boot_part.root_dir();

        for (path, f) in iter {
            match f {
                ContentType::Dir => {
                    root.open_dir(&path).unwrap();
                }
                ContentType::Reader(mut read) => {
                    let mut dst = root.open_file(&path).unwrap();
                    let mut expected = Vec::new();
                    let mut actual = Vec::new();

                    read.read_to_end(&mut expected).unwrap();
                    dst.read_to_end(&mut actual).unwrap();

                    assert_eq!(actual, expected);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_cancellation_respected() {
        let cancel = CancellationToken::default();
        drop(cancel.drop_guard());

        let iter = MockArchive::default();
        let mut sd = MockSd::new();

        let result = internal(iter.into_iter(), &mut sd, Some(cancel));
        assert!(
            matches!(result.unwrap_err(), crate::Error::Aborted),
            "Expected flashing to fail due to cancellation"
        );
    }

    #[test]
    fn test_image_closure_error() {
        let sd = MockSd::new();
        let failing_img_closure = || -> io::Result<MockArchive> {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Image archive missing",
            ))
        };

        let result = common(failing_img_closure, sd, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_immediate_storage_failure() {
        let mut iter = MockArchive::default();
        let mut sd = MockSd::new();

        // Break the device immediately before passing it in
        drop(sd.fail.drop_guard());

        let result = internal((&mut iter).into_iter(), &mut sd, None);

        assert!(
            result.is_err(),
            "Expected an error due to dead storage device"
        );
    }

    #[test]
    fn test_mid_flight_storage_failure() {
        let mut sd = MockSd::new();
        let fail_handle = sd.fail.drop_guard();
        let mut archive = MockArchive::default();

        // Two-way synchronization channels just to orchestrate the steps
        let (signal_tx, signal_rx) = std::sync::mpsc::channel::<()>();
        let (ack_tx, ack_rx) = std::sync::mpsc::channel::<()>();

        std::thread::scope(|s| {
            // Background thread acts as our hardware-pull monkey
            s.spawn(move || {
                // Wait until the main thread tells us it's past the first item
                if signal_rx.recv().is_ok() {
                    // Trip the wire!
                    drop(fail_handle);
                    // Tell the main thread it's safe to proceed with the next items
                    let _ = ack_tx.send(());
                }
            });

            // We wrap the iterator on the MAIN thread. No Send bounds broken!
            let mut step = 0;
            let triggering_iter = (&mut archive).into_iter().map(move |item| {
                if step == 1 {
                    // Signal the background thread to kill the drive
                    let _ = signal_tx.send(());
                    // Block until the background thread confirms the drive is dead
                    let _ = ack_rx.recv();
                }
                step += 1;
                item
            });

            let result = internal(triggering_iter, &mut sd, None);

            assert!(
                result.is_err(),
                "Expected flashing to fail midway through writing"
            );
        });
    }
}
