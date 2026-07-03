#![cfg(feature = "reader_progress")]

use std::io::{Read, Seek, SeekFrom};
use std::sync::mpsc;

use bb_helper::reader_progress::ReaderWithProgress;

#[test]
fn test_happy_path_progress() {
    let data = vec![0u8; 100];
    let (tx, rx) = mpsc::sync_channel(10);

    let mut reader = ReaderWithProgress::new(std::io::Cursor::new(data), 100, Some(tx));
    let mut buf = vec![0u8; 25];

    // Read 1st chunk (25%)
    assert!(reader.read(&mut buf).is_ok());
    assert_eq!(rx.try_recv().unwrap(), 0.25);

    // Read 2nd chunk (50%)
    assert!(reader.read(&mut buf).is_ok());
    assert_eq!(rx.try_recv().unwrap(), 0.50);
}

#[test]
fn test_forward_seek_desync() {
    let data = vec![0u8; 100];
    let (tx, rx) = mpsc::sync_channel(10);

    let mut reader = ReaderWithProgress::new(std::io::Cursor::new(data), 100, Some(tx));
    let mut buf = vec![0u8; 10];

    // 1. Read 10 bytes -> pos should be 10 (10%)
    let count = reader.read(&mut buf).unwrap();
    assert_eq!(count, 10);
    assert_eq!(rx.try_recv().unwrap(), 0.10);

    // 2. Simulate your forward seek (skipping 40 bytes)
    reader.seek(SeekFrom::Current(40)).unwrap();

    // 3. Read another 10 bytes.
    // Real position is now 60 (50 skipped + 10 read), so progress should ideally be 60%.
    let count = reader.read(&mut buf).unwrap();
    assert_eq!(count, 10);

    let reported_progress = rx.try_recv().unwrap();

    // This assertion will FAIL on your current code because your `pos` will
    // think it's only at 20 (10 + 10), reporting 20% instead of 60%.
    assert_eq!(
        reported_progress, 0.60,
        "Progress desynced! Expected 0.60, but got {}",
        reported_progress
    );
}

#[test]
fn test_zero_size_handling() {
    let data = vec![];
    let (tx, rx) = mpsc::sync_channel(10);

    // If someone passes size 0 (e.g., an empty file)
    let mut reader = ReaderWithProgress::new(std::io::Cursor::new(data), 0, Some(tx));
    let mut buf = vec![0u8; 10];

    // This shouldn't panic, but let's check what it emits
    let _ = reader.read(&mut buf);

    if let Ok(progress) = rx.try_recv() {
        // If this is NaN, this assertion will fail because NaN != NaN
        assert!(!progress.is_nan(), "Progress emitted NaN!");
    }
}

#[test]
fn test_dropped_receiver_does_not_panic() {
    let data = vec![0u8; 10];
    let (tx, rx) = mpsc::sync_channel(1);

    let mut reader = ReaderWithProgress::new(std::io::Cursor::new(data), 10, Some(tx));
    let mut buf = vec![0u8; 5];

    // Explicitly drop the receiver side
    drop(rx);

    // This should succeed cleanly because of your `let _ = ` pattern
    assert!(reader.read(&mut buf).is_ok());
}
