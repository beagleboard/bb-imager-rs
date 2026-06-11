use crate::flashing::{BUFFER_SIZE, read_aligned};

use super::write_sd;

fn test_file(len: usize) -> std::io::Cursor<Box<[u8]>> {
    let data: Vec<u8> = (0..len)
        .map(|x| x % 255)
        .map(|x| u8::try_from(x).unwrap())
        .collect();
    std::io::Cursor::new(data.into())
}

#[test]
fn sd_write() {
    const FILE_LEN: usize = 12 * 1024;

    let dummy_file = test_file(FILE_LEN);
    let mut sd = std::io::Cursor::new(Vec::<u8>::new());

    write_sd(
        dummy_file.clone(),
        FILE_LEN as u64,
        None,
        &mut sd,
        None,
        None,
    )
    .unwrap();

    assert_eq!(sd.get_ref().as_slice(), dummy_file.get_ref().as_ref());
}

#[test]
fn sd_write_bmap() {
    const BLOCK_LEN: u64 = BUFFER_SIZE as u64;
    const FILE_LEN: usize = 32 * BUFFER_SIZE;
    const BLOCKS: u64 = (FILE_LEN as u64) / BLOCK_LEN;
    const MAPPED_BLOCKS: &[u64] = &[0, 2, BLOCKS - 1];

    let dummy_file = test_file(FILE_LEN);
    let mut sd = std::io::Cursor::new(vec![0u8; FILE_LEN]);

    let mut bmap = bb_bmap_parser::Bmap::builder();
    bmap.image_size(FILE_LEN as u64)
        .block_size(BLOCK_LEN)
        .blocks(BLOCKS)
        .mapped_blocks(MAPPED_BLOCKS.len() as u64)
        .checksum_type(bb_bmap_parser::HashType::Sha256);

    for i in MAPPED_BLOCKS {
        bmap.add_block_range(
            *i,
            *i,
            bb_bmap_parser::HashValue::Sha256(Default::default()),
        );
    }

    let bmap = bmap.build().unwrap();

    write_sd(
        dummy_file.clone(),
        FILE_LEN as u64,
        Some(bmap.clone()),
        &mut sd,
        None,
        None,
    )
    .unwrap();

    for i in 0..(BLOCKS as usize) {
        let start = i * (BLOCK_LEN as usize);
        let end = start + (BLOCK_LEN as usize);
        if MAPPED_BLOCKS.contains(&(i as u64)) {
            assert_eq!(
                sd.get_ref().as_slice()[start..end],
                dummy_file.get_ref().as_ref()[start..end]
            );
        } else {
            assert_eq!(
                &sd.get_ref().as_slice()[start..end],
                [0u8; BLOCK_LEN as usize].as_slice()
            );
        }
    }
}

#[test]
fn aligned_read() {
    const FILE_LEN: usize = 12 * 1024;

    let mut dummy_file = test_file(FILE_LEN);
    let mut buf = [0u8; 1024];
    let mut pos = 0;

    loop {
        let count = read_aligned(&mut dummy_file, &mut buf).unwrap();
        if count == 0 {
            break;
        }

        assert_eq!(count % 512, 0);
        assert_eq!(buf[..count], dummy_file.get_ref()[pos..(pos + count)]);
        pos += count;
    }

    assert_eq!(pos, FILE_LEN);
}
