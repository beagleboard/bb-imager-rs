use futures::channel::mpsc;

use crate::Result;

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<f32>>, msg: f32) {
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
