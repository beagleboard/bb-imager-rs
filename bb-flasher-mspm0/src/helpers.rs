use std::borrow::Cow;

use tokio::sync::mpsc;

use crate::Status;

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

pub(crate) fn parse_bin<'a>(
    data: &'a [u8],
    fw_size: usize,
) -> Result<Cow<'a, [u8]>, bin_file::Error> {
    match std::str::from_utf8(data) {
        Ok(s) => {
            let t: bin_file::BinFile = s.parse()?;
            t.to_bytes(0..fw_size, Some(0xff)).map(Cow::Owned)
        }
        _ => {
            if fw_size == data.len() {
                Ok(Cow::Borrowed(data))
            } else {
                Err(bin_file::Error::InvalideSize)
            }
        }
    }
}
