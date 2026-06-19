use crate::{Error, Result};
use bb_helper::cancel::CancellationToken;
use fatfs::FileSystem;
use fscommon::{BufStream, StreamSlice};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParitionType {
    Boot,
}

impl ParitionType {
    pub(crate) fn open<T>(&self, dst: T) -> Result<FileSystem<BufStream<StreamSlice<T>>>>
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
        let part_table = PartitionTable::detect_partition_table(&mut dst)?;
        dst.rewind()?;
        let (start_offset, end_offset) = match part_table {
            PartitionTable::Gpt => {
                let disk = gpt::GptConfig::new()
                    .writable(false)
                    .open_from_device(&mut dst)
                    .map_err(|_| crate::Error::InvalidPartitionTable)?;

                let partition_2 = disk.partitions().get(&2).unwrap();

                let start_offset: u64 =
                    partition_2.first_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();
                let end_offset: u64 =
                    partition_2.last_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();

                (start_offset, end_offset)
            }
            PartitionTable::Mbr => {
                let mbr = mbrman::MBRHeader::read_from(&mut dst)
                    .map_err(|_| Error::InvalidPartitionTable)?;

                let boot_part = mbr.get(1).ok_or(Error::InvalidPartitionTable)?;
                let start_offset: u64 = (boot_part.starting_lba * 512).into();
                let end_offset: u64 = start_offset + u64::from(boot_part.sectors) * 512;

                (start_offset, end_offset)
            }
        };

        let slice = StreamSlice::new(dst, start_offset, end_offset)
            .map_err(|_| Error::InvalidPartitionTable)?;
        let boot_stream = BufStream::new(slice);
        FileSystem::new(boot_stream, fatfs::FsOptions::new())
            .map_err(|_| Error::InvalidBootPartition)
    }
}

#[derive(Debug)]
enum PartitionTable {
    Gpt,
    Mbr,
}

impl PartitionTable {
    fn detect_partition_table(mut reader: impl Read) -> Result<PartitionTable> {
        // Read first 1024 bytes (enough for MBR + GPT header)
        let mut buf = [0u8; 1024];
        reader.read_exact(&mut buf)?;

        // Check GPT signature at LBA1 (offset 512)
        if &buf[512..520] == b"EFI PART" {
            return Ok(PartitionTable::Gpt);
        }

        // Check MBR boot signature
        if buf[510] == 0x55 && buf[511] == 0xAA {
            return Ok(PartitionTable::Mbr);
        }

        Err(crate::Error::InvalidPartitionTable)
    }
}

pub enum ContentType<'a> {
    Dir,
    Reader(Box<dyn Read + 'a>),
    File(Box<std::path::Path>),
    DataAppend(Box<[u8]>),
}

impl<'a> From<Box<[u8]>> for ContentType<'a> {
    fn from(value: Box<[u8]>) -> Self {
        Self::DataAppend(value)
    }
}

impl<'a> From<Box<std::path::Path>> for ContentType<'a> {
    fn from(value: Box<std::path::Path>) -> Self {
        Self::File(value)
    }
}

#[derive(Clone, Debug)]
pub struct Customization<I> {
    pub partition: ParitionType,
    pub content: I,
}

impl<'a, I> Customization<I>
where
    I: Iterator<Item = (Box<str>, ContentType<'a>)>,
{
    pub(crate) fn customize(
        self,
        dst: impl Write + Seek + Read + std::fmt::Debug,
        cancel: Option<CancellationToken>,
    ) -> Result<()> {
        let partition = self.partition.open(dst)?;
        {
            let root = partition.root_dir();

            for (path, data) in self.content {
                let customization_err = |source| Error::CustomizationFileCreateFail {
                    source,
                    file: path.clone(),
                };
                crate::helpers::check_cancel(cancel.as_ref())?;

                match data {
                    ContentType::File(spath) => {
                        let mut f = root.create_file(&path).map_err(customization_err)?;
                        let mut source = std::fs::File::open(spath)?;
                        std::io::copy(&mut source, &mut f)?;
                    }
                    ContentType::DataAppend(items) => {
                        let mut f = root.create_file(&path).map_err(customization_err)?;
                        f.seek(SeekFrom::End(0))?;
                        f.write_all(&items)?;
                    }
                    ContentType::Dir => {
                        root.create_dir(&path)?;
                    }
                    ContentType::Reader(mut reader) => {
                        let mut dst = root.create_file(&path).map_err(customization_err)?;
                        dst.truncate()?;
                        std::io::copy(&mut reader, &mut dst)?;
                    }
                }
            }
        }

        partition.unmount()?;

        Ok(())
    }
}
