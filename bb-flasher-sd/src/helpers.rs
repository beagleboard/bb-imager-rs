use std::io::Read;

use futures::channel::mpsc;
use sha2::{Digest, Sha256};

use crate::{Result, Status};

pub(crate) fn sha256_reader_progress(
    mut reader: impl Read,
    size: u64,
    mut chan: Option<futures::channel::mpsc::Sender<Status>>,
) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; 512];
    let mut pos = 0;

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }

        hasher.update(&buffer[..count]);

        pos += count;
        chan_send(chan.as_mut(), Status::Verifying(progress(pos, size)));
    }

    let hash = hasher
        .finalize()
        .as_slice()
        .try_into()
        .expect("SHA-256 is 32 bytes");

    Ok(hash)
}

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<Status>>, msg: Status) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub(crate) const fn progress(pos: usize, img_size: u64) -> f32 {
    pos as f32 / img_size as f32
}

pub(crate) fn check_arc(cancel: Option<&std::sync::Weak<()>>) -> Result<()> {
    match cancel {
        Some(x) if x.strong_count() == 0 => Err(crate::Error::Aborted),
        _ => Ok(()),
    }
}

pub(crate) trait Eject {
    fn eject(self) -> std::io::Result<()>;
}
