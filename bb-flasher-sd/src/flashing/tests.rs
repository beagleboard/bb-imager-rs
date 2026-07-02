use std::{io::Cursor, sync::mpsc};

use crate::flashing::{BUFFER_SIZE, read_aligned};

use super::*;

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
fn test_read_aligned_exact_multiple() {
    let input_data = vec![1u8; 1024]; // Exactly 2x 512-byte alignment blocks
    let mut cursor = Cursor::new(input_data);
    let mut buf = vec![0u8; 1024];

    let result = read_aligned(&mut cursor, &mut buf);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1024);
    assert_eq!(&buf[..], &vec![1u8; 1024][..]);
}

#[test]
fn test_read_aligned_padding_needed() {
    let input_data = vec![5u8; 300]; // Not an alignment multiple (512)
    let mut cursor = Cursor::new(input_data);
    let mut buf = vec![0u8; 512];

    let result = read_aligned(&mut cursor, &mut buf);
    assert!(result.is_ok());
    // It should pad out to the next 512-byte alignment boundary
    assert_eq!(result.unwrap(), 512);

    // Original data intact
    assert_eq!(&buf[0..300], &vec![5u8; 300][..]);
    // Padded area zeroed out
    assert_eq!(&buf[300..512], &vec![0u8; 212][..]);
}

#[test]
fn test_reader_task_stops_at_eof() {
    let input_data = vec![42u8; 100];
    let mut cursor = Cursor::new(input_data);

    let (buf_tx_pool, buf_rx_pool) = mpsc::channel();
    let (buf_tx_out, buf_rx_out) = mpsc::sync_channel(2);

    // Supply one buffer to the pool
    buf_tx_pool.send(Box::new(DirectIoBuffer::new())).unwrap();

    // Run the task (it should read, send data, and then wait or finish if buffer pool empties)
    // Dropping the pool transmitter ensures the loop terminates when buffers run out or EOF hits
    drop(buf_tx_pool);

    let result = reader_task(&mut cursor, buf_rx_pool, buf_tx_out, None);
    assert!(result.is_ok());

    // Verify data reached the output channel
    let (received_buf, count) = buf_rx_out.recv().unwrap();
    // Since input was 100 bytes, it got aligned up to 512
    assert_eq!(count, 512);
    assert_eq!(received_buf.as_slice()[0], 42);
}

#[test]
fn test_writer_task_success() {
    let output = Cursor::new(vec![0u8; 1024]);
    let (tx_out, rx_out) = mpsc::channel();
    let (tx_pool, rx_pool) = mpsc::sync_channel(2);
    let (progress_tx, progress_rx) = mpsc::sync_channel(2);

    // Prep a buffer with data to write
    let mut mock_buf = Box::new(DirectIoBuffer::new());
    mock_buf.as_mut_slice()[0..10].copy_from_slice(&[9u8; 10]);

    tx_out.send((mock_buf, 10)).unwrap();
    drop(tx_out); // Close input stream for writer loop

    let mut writer_target = output;
    let result = writer_task(
        10,
        &mut writer_target,
        Some(progress_tx),
        rx_out,
        tx_pool,
        None,
    );

    assert!(result.is_ok());

    // Assert content was written correctly
    let written_bytes = writer_target.into_inner();
    assert_eq!(&written_bytes[0..10], &[9u8; 10]);

    // Assert progress tracking worked
    assert!(progress_rx.try_recv().is_ok());
    // Assert buffer was successfully recycled back to tx_pool
    assert!(rx_pool.try_recv().is_ok());
}

#[test]
fn test_cancellation_token() {
    let token = CancellationToken::default();
    drop(token.drop_guard());

    let input_data = vec![0u8; 100];
    let mut cursor = Cursor::new(input_data);
    let (buf_tx_pool, buf_rx_pool) = mpsc::channel();
    let (buf_tx_out, _) = mpsc::sync_channel(2);

    buf_tx_pool.send(Box::new(DirectIoBuffer::new())).unwrap();

    let result = reader_task(&mut cursor, buf_rx_pool, buf_tx_out, Some(token));

    // Should return an error variant associated with cancellation
    assert!(result.is_err());
}
