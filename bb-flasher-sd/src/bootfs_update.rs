use std::{
    io::{Read, Seek, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{Result, helpers::Eject};

pub enum ContentType<'a> {
    Dir,
    File(Box<dyn Read + 'a>),
}

fn check_cancel(tkn: Option<&AtomicBool>) -> crate::Result<()> {
    if let Some(t) = tkn
        && t.load(Ordering::Relaxed)
    {
        Err(crate::Error::Aborted)
    } else {
        Ok(())
    }
}

pub fn flash<F, I>(img: F, dst: crate::Destination, cancel: Option<Arc<AtomicBool>>) -> Result<()>
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
                .create(true)
                .truncate(true)
                .open(path)?;
            common(img, sd, cancel)
        }
        crate::Destination::SdCard(path) => {
            let sd = tokio::runtime::Handle::current()
                .block_on(async move { crate::pal::open(&path).await })?;
            common(img, sd, cancel)
        }
    }
}

fn common<F, I, S>(img: F, mut sd: S, cancel: Option<Arc<AtomicBool>>) -> Result<()>
where
    F: FnOnce() -> std::io::Result<I>,
    S: Read + Write + Seek + std::fmt::Debug + Eject,
    for<'b> &'b mut I: IntoIterator<Item = (Box<str>, ContentType<'b>)>,
{
    tracing::info!("Opening Image");
    let mut img = img()?;

    check_cancel(cancel.as_deref())?;

    internal((&mut img).into_iter(), &mut sd, cancel)?;

    tracing::info!("Ejecting SD Card");
    tokio::runtime::Handle::current().block_on(async move {
        let _ = sd.eject().await;
    });

    Ok(())
}

fn internal<'a, I, S>(imgs: I, sd: S, cancel: Option<Arc<AtomicBool>>) -> Result<()>
where
    S: Read + Write + Seek + std::fmt::Debug,
    I: 'a,
    I: Iterator<Item = (Box<str>, ContentType<'a>)>,
{
    tracing::info!("Starting bootfs update");
    let mut sd = crate::helpers::DeviceWrapper::new(sd)?;
    {
        let boot_part = crate::customization::ParitionType::Boot.open(&mut sd)?;
        let root = boot_part.root_dir();

        for (path, c) in imgs {
            tracing::info!("Creating {path}");
            check_cancel(cancel.as_deref())?;

            match c {
                ContentType::Dir => {
                    root.create_dir(&path)?;
                }
                ContentType::File(mut reader) => {
                    let mut dst = root.create_file(&path)?;
                    dst.truncate()?;
                    std::io::copy(&mut reader, &mut dst)?;
                }
            }
        }
    }

    sd.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::io::SeekFrom;

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
                            ContentType::File(Box::new(std::io::Cursor::new(x.clone()))),
                        ),
                        None => (p.clone(), ContentType::Dir),
                    })
                    .collect::<Vec<Self::Item>>()
                    .into_iter(),
            )
        }
    }

    fn mocksd() -> std::fs::File {
        let mut img = tempfile::tempfile().unwrap();

        img.set_len(DISK_SIZE).unwrap();

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
            let mut partition = img.try_clone().unwrap();

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

        img
    }

    #[test]
    fn basic() {
        let iter = MockArchive::default();
        let mut sd = mocksd();

        internal(iter.clone().into_iter(), &mut sd, None).unwrap();
        sd.rewind().unwrap();

        let boot_part = crate::customization::ParitionType::Boot.open(sd).unwrap();
        let root = boot_part.root_dir();

        for (path, f) in iter {
            match f {
                ContentType::Dir => {
                    root.open_dir(&path).unwrap();
                }
                ContentType::File(mut read) => {
                    let mut dst = root.open_file(&path).unwrap();
                    let mut expected = Vec::new();
                    let mut actual = Vec::new();

                    read.read_to_end(&mut expected).unwrap();
                    dst.read_to_end(&mut actual).unwrap();

                    assert_eq!(actual, expected);
                }
            }
        }
    }
}
