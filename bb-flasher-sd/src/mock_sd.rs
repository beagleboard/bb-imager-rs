use std::io::{self, Read, Seek, SeekFrom, Write};

use fscommon::{BufStream, StreamSlice};
use mbrman::{CHS, MBR, MBRPartitionEntry};

use bb_helper::cancel::CancellationToken;

use crate::ContentType;

const DISK_SIZE: u64 = 128 * 1024 * 1024; // 128 MiB
const SECTOR_SIZE: u32 = 512;
const FIRST_LBA: u32 = 2048;

#[derive(Debug)]
pub struct MockSd {
    file: tempfile::NamedTempFile,
    fail: CancellationToken,
}

impl Default for MockSd {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSd {
    pub fn new() -> Self {
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

    pub fn fail_token(&self) -> CancellationToken {
        self.fail.clone()
    }

    pub fn as_file(&self) -> &std::fs::File {
        self.file.as_file()
    }

    pub fn path(&self) -> &std::path::Path {
        self.file.path()
    }

    pub fn open_boot(&mut self) -> fatfs::FileSystem<BufStream<StreamSlice<&mut Self>>> {
        // The partition table is read from the current cursor.
        self.rewind().unwrap();
        crate::customization::ParitionType::Boot.open(self).unwrap()
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

impl crate::helpers::Eject for MockSd {
    fn eject(self) -> io::Result<()> {
        self.as_file().sync_all()
    }
}

/// Replayable description of one archive entry.
///
/// Mirrors [`ContentType`], but stores reader contents as bytes so the same entry
/// can be handed out again on every iteration.
pub enum MockContent {
    Dir,
    Reader(Box<[u8]>),
    File(Box<std::path::Path>),
    DataAppend(Box<[u8]>),
}

impl MockContent {
    fn as_content_type(&self) -> ContentType<'_> {
        match self {
            Self::Dir => ContentType::Dir,
            Self::Reader(data) => ContentType::Reader(Box::new(data.as_ref())),
            Self::File(path) => ContentType::File(path.clone()),
            Self::DataAppend(data) => ContentType::DataAppend(data.clone()),
        }
    }
}

pub struct MockArchive(Vec<(Box<str>, MockContent)>);

impl MockArchive {
    pub fn from_entries(entries: Vec<(Box<str>, MockContent)>) -> Self {
        Self(entries)
    }
}

impl<'a> IntoIterator for &'a MockArchive {
    type Item = (Box<str>, ContentType<'a>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.0
                .iter()
                .map(|(path, content)| (path.clone(), content.as_content_type())),
        )
    }
}

impl<'a> IntoIterator for &'a mut MockArchive {
    type Item = (Box<str>, ContentType<'a>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        (&*self).into_iter()
    }
}
