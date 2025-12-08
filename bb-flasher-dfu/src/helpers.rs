use std::io;

pub(crate) fn check_token(
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> crate::Result<()> {
    match cancel {
        Some(x) if x.is_cancelled() => Err(crate::Error::Aborted),
        _ => Ok(()),
    }
}

pub(crate) fn is_dfu_device<U: rusb::UsbContext>(x: &rusb::Device<U>) -> bool {
    if let Ok(cfg_desc) = x.active_config_descriptor() {
        for intf in cfg_desc.interfaces() {
            for desc in intf.descriptors() {
                if desc.class_code() == 0xfe && desc.sub_class_code() == 1 {
                    return true;
                }
            }
        }
    }

    false
}

pub(crate) struct ReaderWithProgress<R: std::io::Read> {
    reader: R,
    pos: u64,
    size: u64,
    chan: tokio::sync::mpsc::Sender<f32>,
}

impl<R: std::io::Read> ReaderWithProgress<R> {
    pub(crate) const fn new(reader: R, size: u64, chan: tokio::sync::mpsc::Sender<f32>) -> Self {
        Self {
            reader,
            size,
            chan,
            pos: 0,
        }
    }
}

impl<R: std::io::Read> std::io::Read for ReaderWithProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(buf)?;

        self.pos += count as u64;
        let _ = self.chan.try_send(self.pos as f32 / self.size as f32);

        Ok(count)
    }
}
