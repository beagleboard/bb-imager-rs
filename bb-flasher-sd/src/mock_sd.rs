use std::io::{self, Read, Seek, SeekFrom, Write};

use fscommon::{BufStream, StreamSlice};
use mbrman::{CHS, MBR, MBRPartitionEntry};

use bb_helper::cancel::CancellationToken;

use crate::ContentType;

const DISK_SIZE: u64 = 128 * 1024 * 1024; // 128 MiB
const SECTOR_SIZE: u32 = 512;
const FIRST_LBA: u32 = 2048;

/// EFI System Partition type GUID (C12A7328-F81F-11D2-BA4B-00A0C93EC93B),
/// in the mixed-endian byte order gptman expects.
const EFI_SYSTEM_PARTITION_GUID: [u8; 16] = [
    0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
];
const DISK_GUID: [u8; 16] = [
    0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
];
const PARTITION_GUID: [u8; 16] = [
    0x87, 0x65, 0x43, 0x21, 0xcb, 0xa9, 0x0f, 0xed, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
];

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
        format_fat32(&img, partition_offset, partition_size);

        img.rewind().unwrap();
        Self {
            file: img,
            fail: CancellationToken::default(),
        }
    }

    pub fn new_gpt() -> Self {
        let mut img = tempfile::NamedTempFile::new().unwrap();
        img.as_file().set_len(DISK_SIZE).unwrap();

        // Protective MBR, so tools that only understand MBR see one big
        // "GPT protective" partition rather than unpartitioned media.
        gptman::GPT::write_protective_mbr_into(&mut img, SECTOR_SIZE as u64).unwrap();

        let mut gpt = gptman::GPT::new_from(&mut img, SECTOR_SIZE as u64, DISK_GUID).unwrap();

        // Reuse FIRST_LBA for alignment parity with the MBR image; it must sit
        // past the primary header + entry array.
        let starting_lba = FIRST_LBA as u64;
        assert!(starting_lba >= gpt.header.first_usable_lba);
        // Leave room for the backup header/entries at the end of the disk.
        let ending_lba = gpt.header.last_usable_lba;

        gpt[1] = gptman::GPTPartitionEntry {
            partition_type_guid: EFI_SYSTEM_PARTITION_GUID,
            unique_partition_guid: PARTITION_GUID,
            starting_lba,
            ending_lba,
            attribute_bits: 1 << 2,
            partition_name: "BOOT".into(),
        };
        gpt.write_into(&mut img).unwrap();

        let partition_offset = starting_lba * SECTOR_SIZE as u64;
        let partition_size = (ending_lba - starting_lba + 1) * SECTOR_SIZE as u64;
        format_fat32(&img, partition_offset, partition_size);

        img.rewind().unwrap();
        Self {
            file: img,
            fail: CancellationToken::default(),
        }
    }

    pub fn size(&self) -> u64 {
        DISK_SIZE
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

    /// A standalone copy of the current bytes.
    ///
    /// Flashing truncates its destination, so a test that flashes onto this
    /// `MockSd` (the only way [`Self::open_boot`] can inspect the result) needs
    /// its OS image in a separate file.
    pub fn image_copy(&self) -> tempfile::NamedTempFile {
        let mut image = tempfile::NamedTempFile::new().unwrap();
        let mut src = std::fs::File::open(self.path()).unwrap();
        io::copy(&mut src, image.as_file_mut()).unwrap();
        image.flush().unwrap();
        image
    }

    pub fn open_boot(&mut self) -> fatfs::FileSystem<BufStream<StreamSlice<&mut Self>>> {
        // The partition table is read from the current cursor.
        self.rewind().unwrap();
        crate::customization::ParitionType::Boot.open(self).unwrap()
    }

    /// Read a file from the boot partition.
    pub fn boot_file(&mut self, path: &str) -> io::Result<String> {
        let fs = self.open_boot();
        let mut out = String::new();
        fs.root_dir().open_file(path)?.read_to_string(&mut out)?;
        Ok(out)
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

fn format_fat32(img: &tempfile::NamedTempFile, offset: u64, size: u64) {
    let mut partition = img.reopen().unwrap();
    partition.seek(SeekFrom::Start(offset)).unwrap();
    let mut partition = StreamSlice::new(partition, offset, offset + size).unwrap();
    fatfs::format_volume(
        &mut partition,
        fatfs::FormatVolumeOptions::new()
            .fat_type(fatfs::FatType::Fat32)
            .volume_label(*b"BOOT       "),
    )
    .unwrap();
}
