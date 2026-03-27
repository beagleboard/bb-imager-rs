use crate::{Error, Result};
use fatfs::FileSystem;
use fscommon::{BufStream, StreamSlice};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParitionType {
    Boot,
}

impl ParitionType {
    fn open<T>(&self, dst: T) -> Result<FileSystem<BufStream<StreamSlice<T>>>>
    where
        T: Write + Seek + Read + std::fmt::Debug,
    {
        match self {
            Self::Boot => Self::boot_partition(dst),
        }
    }

    fn boot_partition<T>(mut dst: T) -> Result<FileSystem<BufStream<StreamSlice<T>>>>
    where
        T: Write + Seek + Read + std::fmt::Debug,
    {
        // First try GPT partition table. If that fails, try MBR
        let (start_offset, end_offset) = if let Ok(disk) = gpt::GptConfig::new()
            .writable(false)
            .open_from_device(&mut dst)
        {
            // FIXME: Add better partition lookup
            let partition_2 = disk.partitions().get(&2).unwrap();

            let start_offset: u64 = partition_2.first_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();
            let end_offset: u64 = partition_2.last_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();

            (start_offset, end_offset)
        } else {
            let mbr =
                mbrman::MBRHeader::read_from(&mut dst).map_err(|_| Error::InvalidPartitionTable)?;

            let boot_part = mbr.get(1).ok_or(Error::InvalidPartitionTable)?;
            let start_offset: u64 = (boot_part.starting_lba * 512).into();
            let end_offset: u64 = start_offset + u64::from(boot_part.sectors) * 512;

            (start_offset, end_offset)
        };
        let slice = StreamSlice::new(dst, start_offset, end_offset)
            .map_err(|_| Error::InvalidPartitionTable)?;
        let boot_stream = BufStream::new(slice);
        FileSystem::new(boot_stream, fatfs::FsOptions::new())
            .map_err(|_| Error::InvalidBootPartition)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ContentType {
    File(Box<std::path::Path>),
    Data(Box<[u8]>),
}

impl From<Box<[u8]>> for ContentType {
    fn from(value: Box<[u8]>) -> Self {
        Self::Data(value)
    }
}

impl From<Box<std::path::Path>> for ContentType {
    fn from(value: Box<std::path::Path>) -> Self {
        Self::File(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Customization {
    pub partition: ParitionType,
    pub content: Vec<(Box<str>, ContentType)>,
}

impl Customization {
    pub(crate) fn customize(&self, dst: impl Write + Seek + Read + std::fmt::Debug) -> Result<()> {
        let partition = self.partition.open(dst)?;
        let root = partition.root_dir();

        for (path, data) in &self.content {
            let mut f =
                root.create_file(path)
                    .map_err(|source| Error::CustomizationFileCreateFail {
                        source,
                        file: path.clone(),
                    })?;

            match data {
                ContentType::File(path) => {
                    let mut source = std::fs::File::open(path)?;
                    std::io::copy(&mut source, &mut f)?;
                }
                ContentType::Data(items) => {
                    f.seek(SeekFrom::End(0))?;
                    f.write_all(&items)?;
                }
            }
        }

        Ok(())
    }
}
