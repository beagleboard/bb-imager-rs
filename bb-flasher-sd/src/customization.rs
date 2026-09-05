use crate::{Error, Result};
use bb_helper::cancel::CancellationToken;
use fatfs::FileSystem;
use fscommon::{BufStream, StreamSlice};
use std::io::{Read, Seek, SeekFrom, Write};

const GPT_EFI_ATTR: u64 = 1 << 1;
const GPT_BIOS_ATTR: u64 = 1 << 2;

const SECTOR_SIZE: u32 = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParitionType {
    Boot,
}

impl ParitionType {
    pub(crate) fn open<T>(&self, dst: T) -> Result<FileSystem<BufStream<StreamSlice<T>>>>
    where
        T: Write + Seek + Read,
    {
        match self {
            Self::Boot => Self::boot_partition(dst),
        }
    }

    fn boot_partition<T>(mut dst: T) -> Result<FileSystem<BufStream<StreamSlice<T>>>>
    where
        T: Write + Seek + Read,
    {
        let part_table = PartitionTable::detect_partition_table(&mut dst)?;
        dst.rewind()?;
        let (start_offset, end_offset) = match part_table {
            PartitionTable::Gpt => {
                let disk = gptman::GPT::find_from(&mut dst)
                    .map_err(|_| crate::Error::InvalidPartitionTable)?;

                // Find the first bootable partition
                let (_, boot_partition) = disk
                    .iter()
                    .find(|(_, part)| {
                        part.is_used()
                            && (part.attribute_bits & (GPT_EFI_ATTR | GPT_BIOS_ATTR) != 0)
                    })
                    .ok_or(crate::Error::InvalidPartitionTable)?;
                tracing::info!("Found GPT boot partition: {:#?}", boot_partition);

                let start_offset = boot_partition.starting_lba * disk.sector_size;
                // `ending_lba` is inclusive.
                let end_offset = (boot_partition.ending_lba + 1) * disk.sector_size;

                (start_offset, end_offset)
            }
            PartitionTable::Mbr => {
                let mbr = mbrman::MBRHeader::read_from(&mut dst)
                    .map_err(|_| Error::InvalidPartitionTable)?;

                // Find the first bootable partition
                let (_, boot_part) = mbr
                    .iter()
                    .find(|(_, part)| part.is_used() && part.is_active())
                    .ok_or(Error::InvalidPartitionTable)?;

                let start_offset: u64 = (boot_part.starting_lba * SECTOR_SIZE).into();
                let end_offset: u64 = start_offset + u64::from(boot_part.sectors * SECTOR_SIZE);

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
        dst: impl Write + Seek + Read,
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

pub(crate) fn resize_last_partition(dst: impl Read + Write + Seek, total_size: u64) -> Result<()> {
    let mut dst = StreamSlice::new(dst, 0, total_size)?;

    let part_table = PartitionTable::detect_partition_table(&mut dst)?;
    dst.rewind()?;

    match part_table {
        PartitionTable::Gpt => {
            let mut gpt =
                gptman::GPT::find_from(&mut dst).map_err(|_| Error::InvalidPartitionTable)?;
            // The image just grew: last_usable_lba/backup_lba in the on-disk header are stale.
            gpt.header
                .update_from(&mut dst, gpt.sector_size)
                .map_err(|_| Error::InvalidPartitionTable)?;

            let last_usable_lba = gpt.header.last_usable_lba;
            let (_, last_part) = gpt
                .iter_mut()
                .filter(|(_, x)| x.is_used())
                .max_by_key(|(_, x)| x.starting_lba)
                .ok_or(Error::InvalidPartitionTable)?;

            last_part.ending_lba = last_usable_lba;

            dst.rewind()?;
            gpt.write_into(&mut dst)
                .map_err(|_| Error::InvalidPartitionTable)?;
        }
        PartitionTable::Mbr => {
            let mut mbr = mbrman::MBR::read_from(&mut dst, SECTOR_SIZE)
                .map_err(|_| Error::InvalidPartitionTable)?;

            let (id, _) = mbr
                .iter()
                .filter(|(_, x)| x.is_used() && !x.is_extended())
                .max_by_key(|(_, x)| x.starting_lba)
                .ok_or(Error::InvalidPartitionTable)?;

            let new_sectors = mbr
                .get_maximum_partition_size_for(id)
                .map_err(|_| Error::InvalidPartitionTable)?;
            mbr[id].sectors = new_sectors;

            // Rewind is not really needed since write_into internally does it. But well, 2 rewinds
            // won't cause any problem.
            dst.rewind()?;
            mbr.write_into(&mut dst)
                .map_err(|_| Error::InvalidPartitionTable)?;
        }
    }

    dst.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::mock_sd::MockSd;

    use super::*;

    /// Read back the partition table `reseize_last_partition` just wrote.
    fn read_mbr(sd: &mut MockSd) -> mbrman::MBR {
        sd.rewind().unwrap();
        mbrman::MBR::read_from(sd, SECTOR_SIZE).unwrap()
    }

    /// Replace the mock's table with `parts`, given as `(sys, starting_lba,
    /// sectors)`. The FAT32 filesystem the mock formatted is left where it is;
    /// these tests only care about the table.
    fn write_table(sd: &mut MockSd, parts: &[(u8, u32, u32)]) {
        let mut mbr = read_mbr(sd);

        for i in 1..=4 {
            mbr[i] = mbrman::MBRPartitionEntry::empty();
        }
        for (i, (sys, starting_lba, sectors)) in parts.iter().enumerate() {
            mbr[i + 1] = mbrman::MBRPartitionEntry {
                boot: mbrman::BOOT_INACTIVE,
                first_chs: mbrman::CHS::empty(),
                sys: *sys,
                last_chs: mbrman::CHS::empty(),
                starting_lba: *starting_lba,
                sectors: *sectors,
            };
        }

        sd.rewind().unwrap();
        mbr.write_into(sd).unwrap();
    }

    /// The card is bigger than the image it was flashed with, which is the case
    /// the resize exists for: the last partition grows to the end of the card.
    ///
    /// `total_size` is what makes the disk look bigger; only LBA0 is rewritten,
    /// so the mock's file does not have to grow with it.
    #[test]
    fn resize_grows_last_partition_to_fill_the_disk() {
        let mut sd = MockSd::new();
        let total_size = sd.size() * 2;
        let before = read_mbr(&mut sd)[1].sectors;

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let part = read_mbr(&mut sd)[1].clone();
        assert!(
            part.sectors > before,
            "partition should have grown, was {before} sectors"
        );
        assert_eq!(
            u64::from(part.starting_lba + part.sectors) * u64::from(SECTOR_SIZE),
            total_size,
            "partition should reach the end of the card"
        );

        // Only the table is resized, so the filesystem is still readable.
        sd.rewind().unwrap();
        ParitionType::Boot.open(&mut sd).unwrap();
    }

    /// Flashed onto a card exactly as big as the image: there is nothing to grow
    /// into, so the table comes out untouched.
    #[test]
    fn resize_is_noop_when_partition_already_fills_the_disk() {
        let mut sd = MockSd::new();
        let total_size = sd.size();
        let before = read_mbr(&mut sd)[1].clone();

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        assert_eq!(read_mbr(&mut sd)[1], before);
    }

    /// Only the last partition is resized; the ones before it keep their size.
    ///
    /// Both partitions start at a multiple of 2048: `MBR::read_from` derives the
    /// alignment from the starting LBAs, and a partition whose end is not
    /// aligned would silently fail to match the free space behind it.
    #[test]
    fn resize_grows_only_the_last_partition() {
        let mut sd = MockSd::new();
        let total_size = sd.size();
        write_table(&mut sd, &[(0x0C, 2048, 2048), (0x83, 4096, 2048)]);

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let mbr = read_mbr(&mut sd);
        assert_eq!(
            mbr[1].sectors, 2048,
            "the boot partition should be left as is"
        );
        assert_eq!(
            u64::from(mbr[2].starting_lba + mbr[2].sectors) * u64::from(SECTOR_SIZE),
            total_size,
            "the last partition should reach the end of the card"
        );
    }

    /// An extended partition is a container for logical ones, not something to
    /// grow, so the last *real* partition is picked instead. Here that one is
    /// boxed in by the container, which leaves the whole table unchanged.
    #[test]
    fn resize_skips_extended_partitions() {
        let mut sd = MockSd::new();
        let total_size = sd.size();
        write_table(&mut sd, &[(0x0C, 2048, 2048), (0x05, 4096, 2048)]);

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let mbr = read_mbr(&mut sd);
        assert_eq!(mbr[1].sectors, 2048);
        assert_eq!(
            mbr[2].sectors, 2048,
            "an extended partition should not be resized"
        );
    }

    /// Read back the GPT `resize_last_partition` just wrote.
    fn read_gpt(sd: &mut MockSd) -> gptman::GPT {
        sd.rewind().unwrap();
        gptman::GPT::find_from(sd).unwrap()
    }

    /// Replace the mock's GPT entries with `parts`, given as `(starting_lba,
    /// ending_lba)` (both inclusive). The FAT32 filesystem the mock formatted is
    /// left where it is; these tests only care about the table.
    fn write_gpt_table(sd: &mut MockSd, parts: &[(u64, u64)]) {
        let mut gpt = read_gpt(sd);

        for i in 1..=gpt.header.number_of_partition_entries {
            gpt[i] = gptman::GPTPartitionEntry::empty();
        }
        for (i, (starting_lba, ending_lba)) in parts.iter().enumerate() {
            let n = i as u32 + 1;
            gpt[n] = gptman::GPTPartitionEntry {
                // Any non-zero type marks the entry as used.
                partition_type_guid: [0xaf; 16],
                // The GUIDs have to differ or `write_into` rejects the table.
                unique_partition_guid: [n as u8; 16],
                starting_lba: *starting_lba,
                ending_lba: *ending_lba,
                attribute_bits: 0,
                partition_name: "".into(),
            };
        }

        sd.rewind().unwrap();
        gpt.write_into(sd).unwrap();
    }

    /// GPT counterpart of [`resize_grows_last_partition_to_fill_the_disk`]: the
    /// card is bigger than the image it was flashed with, so the last partition
    /// grows into the space behind it.
    #[test]
    fn resize_grows_last_gpt_partition_to_fill_the_disk() {
        let mut sd = MockSd::new_gpt();
        let total_size = sd.size() * 2;
        let before = read_gpt(&mut sd)[1].ending_lba;

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let gpt = read_gpt(&mut sd);
        assert!(
            gpt[1].ending_lba > before,
            "partition should have grown, ended at LBA {before}"
        );
        assert_eq!(
            gpt[1].ending_lba, gpt.header.last_usable_lba,
            "partition should reach the end of the usable area"
        );

        // Only the table is resized, so the filesystem is still readable.
        sd.rewind().unwrap();
        ParitionType::Boot.open(&mut sd).unwrap();
    }

    /// The usable area is bounded by the backup header, which sits in the last
    /// sector of the card. Unless `resize_last_partition` moves it there from the
    /// end of the (smaller) image, the partition cannot grow past the old end.
    #[test]
    fn resize_moves_the_backup_gpt_to_the_end_of_the_disk() {
        let mut sd = MockSd::new_gpt();
        let total_size = sd.size() * 2;

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let gpt = read_gpt(&mut sd);
        assert_eq!(
            (gpt.header.backup_lba + 1) * gpt.sector_size,
            total_size,
            "the backup header should be in the last sector of the card"
        );

        // ... and it should actually be there, not just be pointed at.
        let mut signature = [0u8; 8];
        sd.seek(SeekFrom::Start(gpt.header.backup_lba * gpt.sector_size))
            .unwrap();
        sd.read_exact(&mut signature).unwrap();
        assert_eq!(&signature, b"EFI PART");
    }

    /// Flashed onto a card exactly as big as the image: there is nothing to grow
    /// into, so the table comes out untouched.
    #[test]
    fn resize_is_noop_when_gpt_partition_already_fills_the_disk() {
        let mut sd = MockSd::new_gpt();
        let total_size = sd.size();
        let before = read_gpt(&mut sd)[1].clone();

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        assert_eq!(read_gpt(&mut sd)[1], before);
    }

    /// Only the last partition is resized; the ones before it keep their size.
    #[test]
    fn resize_grows_only_the_last_gpt_partition() {
        let mut sd = MockSd::new_gpt();
        let total_size = sd.size();
        write_gpt_table(&mut sd, &[(2048, 4095), (4096, 6143)]);

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let gpt = read_gpt(&mut sd);
        assert_eq!(
            gpt[1].ending_lba, 4095,
            "the boot partition should be left as is"
        );
        assert_eq!(
            gpt[2].ending_lba, gpt.header.last_usable_lba,
            "the last partition should reach the end of the usable area"
        );
    }

    /// "Last" means furthest into the disk, not last in the entry array: GPT
    /// entries are free to be stored out of order.
    #[test]
    fn resize_grows_the_furthest_gpt_partition_not_the_last_entry() {
        let mut sd = MockSd::new_gpt();
        let total_size = sd.size();
        write_gpt_table(&mut sd, &[(4096, 6143), (2048, 4095)]);

        sd.rewind().unwrap();
        resize_last_partition(&mut sd, total_size).unwrap();

        let gpt = read_gpt(&mut sd);
        assert_eq!(
            gpt[1].ending_lba, gpt.header.last_usable_lba,
            "the partition furthest into the disk should have grown"
        );
        assert_eq!(
            gpt[2].ending_lba, 4095,
            "the first partition on disk should be left as is"
        );
    }

    #[test]
    fn resize_rejects_a_gpt_without_any_partition() {
        let mut sd = MockSd::new_gpt();
        let total_size = sd.size();
        write_gpt_table(&mut sd, &[]);

        sd.rewind().unwrap();
        let err = resize_last_partition(&mut sd, total_size).unwrap_err();

        assert!(
            matches!(err, Error::InvalidPartitionTable),
            "expected InvalidPartitionTable, got: {err:?}"
        );
    }

    #[test]
    fn resize_rejects_an_image_without_a_partition_table() {
        let mut img = std::io::Cursor::new(vec![0u8; 1024]);

        let err = resize_last_partition(&mut img, 1024).unwrap_err();

        assert!(
            matches!(err, Error::InvalidPartitionTable),
            "expected InvalidPartitionTable, got: {err:?}"
        );
    }
}
