use futures::channel::mpsc;

use crate::Status;

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<Status>>, msg: Status) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub(crate) fn parse_bin(data: &[u8]) -> Result<bin_file::BinFile, bin_file::Error> {
    const THRESHOLD: usize = 20;

    match std::str::from_utf8(data) {
        Ok(s) => s.parse(),
        _ => bin_file_from_binary(data, THRESHOLD),
    }
}

fn bin_file_from_binary(
    data: &[u8],
    threshold: usize,
) -> Result<bin_file::BinFile, bin_file::Error> {
    let mut offset = 0;
    let mut binfile = bin_file::BinFile::new();

    assert!(threshold > 0);

    while offset < data.len() {
        let sendable = data[offset..].iter().take_while(|x| **x != 0xff).count();
        let skippable = data[offset + sendable..]
            .iter()
            .take_while(|x| **x == 0xff)
            .count();

        let end = if skippable > threshold {
            let temp = offset + sendable;
            temp + (temp & 1)
        } else {
            offset + sendable + skippable
        };

        binfile
            .add_bytes(&data[offset..end], Some(offset), false)
            .unwrap();
        offset += sendable + skippable;
    }

    Ok(binfile)
}
