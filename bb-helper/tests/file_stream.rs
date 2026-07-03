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
