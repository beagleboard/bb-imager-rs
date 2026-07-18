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
fn test_progress_tracks_absolute_position_after_seek() {
    let data = vec![0u8; 100];
    let (tx, rx) = mpsc::sync_channel(10);

    let mut reader = ReaderWithProgress::new(std::io::Cursor::new(data), 100, Some(tx));
    let mut buf = vec![0u8; 10];

    // 1. Read 10 bytes -> pos should be 10 (10%)
    let count = reader.read(&mut buf).unwrap();
    assert_eq!(count, 10);
    assert_eq!(rx.try_recv().unwrap(), 0.10);

    // 2. Forward seek, skipping 40 bytes -> absolute position 50.
    reader.seek(SeekFrom::Current(40)).unwrap();

    // 3. Read another 10 bytes. Real position is now 60.
    let count = reader.read(&mut buf).unwrap();
    assert_eq!(count, 10);

    let reported_progress = rx.try_recv().unwrap();

    // The Seek impl re-syncs `pos` to the reader's absolute position
    // (`self.pos = self.reader.seek(pos)?`), so progress after a seek stays
    // accurate: 60%, not the 20% a naive read-only byte counter would report.
    assert_eq!(
        reported_progress, 0.60,
        "Progress should track absolute position after a seek, got {}",
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
