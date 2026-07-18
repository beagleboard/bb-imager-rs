//! Integration tests for `Downloader::download_to_stream`, the streaming +
//! SHA256-verifying download path. This is the crate's biggest untested public
//! surface: the existing tests cover URL-cache download and cache lookup, but
//! nothing exercises the stream/verify/persist flow or its rejection branch.

use bb_downloader::Downloader;
use bb_helper::file_stream::file_stream;
use httpmock::{Method::GET, MockServer};
use sha2::{Digest, Sha256};
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Return the single file in `dir`, or None if empty.
fn single_file(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .next()
}

#[tokio::test]
async fn download_to_stream_persists_on_sha_match() {
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let downloader = Downloader::new(tmp.path()).unwrap();

    let content = b"streamed payload bytes";
    let sha = sha256(content);

    let mock = server.mock(|when, then| {
        when.method(GET).path("/img");
        then.status(200).body(content);
    });

    let (writer, _reader) = file_stream().unwrap();
    downloader
        .download_to_stream(server.url("/img"), sha, writer)
        .await
        .expect("matching sha should succeed");

    mock.assert_calls(1);

    let path = single_file(tmp.path()).expect("a file should be persisted");
    assert_eq!(std::fs::read(&path).unwrap(), content);
    // Persisted under the SHA-derived cache name.
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        const_hex::encode(sha)
    );
}

#[tokio::test]
async fn download_to_stream_rejects_sha_mismatch() {
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let downloader = Downloader::new(tmp.path()).unwrap();

    let content = b"streamed payload bytes";
    // A hash that cannot match the served content.
    let wrong_sha = [0u8; 32];

    let mock = server.mock(|when, then| {
        when.method(GET).path("/img");
        then.status(200).body(content);
    });

    let (writer, _reader) = file_stream().unwrap();
    let err = downloader
        .download_to_stream(server.url("/img"), wrong_sha, writer)
        .await
        .expect_err("mismatched sha must fail");

    mock.assert_calls(1);
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    assert!(
        single_file(tmp.path()).is_none(),
        "no file should be persisted when the checksum does not match"
    );
}
