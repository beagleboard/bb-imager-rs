use std::io::{Read, Seek, Write};

use crate::{
    Result,
    helpers::{Eject, check_token},
};

pub enum FileType<'a> {
    Dir,
    File(Box<dyn Read + 'a>),
}

pub async fn update_boot<I>(
    img: impl Future<Output = std::io::Result<I>>,
    dst: crate::Destination,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()>
where
    I: Send + 'static,
    for<'b> &'b mut I: IntoIterator<Item = (Box<str>, FileType<'b>)>,
{
    tracing::info!("Opening Destination");

    match dst {
        crate::Destination::File(path) => {
            let sd = tokio::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .await?
                .into_std()
                .await;
            common(img, sd, cancel).await
        }
        crate::Destination::SdCard(path) => {
            let sd = crate::pal::open(&path).await?;
            common(img, sd, cancel).await
        }
    }
}

async fn common<I>(
    img: impl Future<Output = std::io::Result<I>>,
    sd: impl Read + Write + Seek + Eject + std::fmt::Debug + Send + 'static,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()>
where
    I: Send + 'static,
    for<'b> &'b mut I: IntoIterator<Item = (Box<str>, FileType<'b>)>,
{
    let img = img.await?;

    let cancel_child = cancel.as_ref().map(|x| x.child_token());
    let res = tokio::task::spawn_blocking(move || internal(img, sd, cancel_child))
        .await
        .unwrap();

    // Cancel all tasks on drop
    let _drop_guard = cancel.map(|x| x.drop_guard());

    res
}

fn internal<I>(
    mut boot: I,
    mut sd: impl Read + Write + Seek + Eject + std::fmt::Debug,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()>
where
    for<'b> &'b mut I: IntoIterator<Item = (Box<str>, FileType<'b>)>,
{
    check_token(cancel.as_ref())?;

    tracing::info!("Updating bootfs");
    {
        let boot_part = crate::customization::ParitionType::Boot
            .open(crate::helpers::DeviceWrapper::new(&mut sd)?)?;
        let root = boot_part.root_dir();
        for (path, c) in &mut boot {
            match c {
                FileType::Dir => {
                    root.create_dir(&path)?;
                }
                FileType::File(mut reader) => {
                    let mut dst = root.create_file(&path)?;
                    dst.truncate()?;
                    std::io::copy(&mut reader, &mut dst)?;
                }
            }
        }
    }
    sd.flush()?;

    tracing::info!("Ejecting SD Card");
    let _ = sd.eject();

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

    impl<'a> IntoIterator for &'a mut MockArchive {
        type Item = (Box<str>, FileType<'a>);
        type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

        fn into_iter(self) -> Self::IntoIter {
            Box::new(self.0.iter().map(|(p, f)| match f {
                Some(x) => (p.clone(), FileType::File(Box::new(x.as_slice()))),
                None => (p.clone(), FileType::Dir),
            }))
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
        let mut iter = MockArchive::default();
        let mut sd = mocksd();

        internal(iter.clone(), &mut sd, None).unwrap();
        sd.rewind().unwrap();

        let boot_part = crate::customization::ParitionType::Boot.open(sd).unwrap();
        let root = boot_part.root_dir();

        for (path, f) in &mut iter {
            match f {
                FileType::Dir => {
                    root.open_dir(&path).unwrap();
                }
                FileType::File(mut read) => {
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
