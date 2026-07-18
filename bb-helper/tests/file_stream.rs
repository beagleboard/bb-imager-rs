#![cfg(feature = "file_stream")]

use bb_helper::file_stream::*;
use std::io::{self, Read, Seek, SeekFrom};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn seek_from_start_works() {
    let (mut writer, mut reader) = file_stream().unwrap();

    writer.write_all(b"abcdef").await.unwrap();
    writer.flush().await.unwrap();

    tokio::task::spawn_blocking(move || {
        reader.seek(SeekFrom::Start(2)).unwrap();

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();

        assert_eq!(&buf, b"cd");
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn seek_from_current_works() {
    let (mut writer, mut reader) = file_stream().unwrap();

    writer.write_all(b"abcdef").await.unwrap();
    writer.flush().await.unwrap();

    tokio::task::spawn_blocking(move || {
        reader.seek(SeekFrom::Start(1)).unwrap();
        reader.seek(SeekFrom::Current(2)).unwrap();

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();

        assert_eq!(&buf, b"d");
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn seek_from_end_fails_while_writer_alive() {
    let (_writer, mut reader) = file_stream().unwrap();

    tokio::task::spawn_blocking(move || {
        let err = reader.seek(SeekFrom::End(0)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn seek_from_end_works_after_writer_drop() {
    let (mut writer, mut reader) = file_stream().unwrap();

    writer.write_all(b"abcdef").await.unwrap();
    writer.flush().await.unwrap();
    drop(writer);

    tokio::task::spawn_blocking(move || {
        let pos = reader.seek(SeekFrom::End(-2)).unwrap();
        assert_eq!(pos, 4);

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();

        assert_eq!(&buf, b"ef");
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn invalid_negative_seek_fails() {
    let (mut writer, mut reader) = file_stream().unwrap();

    writer.write_all(b"abc").await.unwrap();
    writer.flush().await.unwrap();

    tokio::task::spawn_blocking(move || {
        let err = reader.seek(SeekFrom::Current(-10)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    })
    .await
    .unwrap()
}

/// The core streaming contract: a blocking reader waits for the writer to
/// produce data (the condvar path in `ReaderFileStream::read`) and observes EOF
/// only once the writer is dropped. Correctness holds regardless of timing; the
/// reader starts first to make it likely the blocking-wait branch is taken.
#[tokio::test]
async fn blocking_read_receives_all_data_then_eof_on_writer_drop() {
    let (mut writer, mut reader) = file_stream().unwrap();

    let reader_task = tokio::task::spawn_blocking(move || {
        let mut out = Vec::new();
        // read_to_end blocks on each empty read until the writer closes.
        reader.read_to_end(&mut out).unwrap();
        out
    });

    // Let the reader start and block on the initially-empty file.
    tokio::task::yield_now().await;

    for chunk in [b"hello ".as_slice(), b"world", b"!"] {
        writer.write_all(chunk).await.unwrap();
        writer.flush().await.unwrap();
        tokio::task::yield_now().await;
    }

    // Dropping the writer flips the shared flag and notifies the reader,
    // turning the next empty read into a clean EOF.
    drop(writer);

    let out = reader_task.await.unwrap();
    assert_eq!(out, b"hello world!");
}

#[tokio::test]
async fn persist_writes_byte_exact_file() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("out.bin");

    let (mut writer, _reader) = file_stream().unwrap();
    writer.write_all(b"persist me").await.unwrap();
    writer.flush().await.unwrap();

    writer.persist(&dest).await.unwrap();

    assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"persist me");
}
