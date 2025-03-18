# BB Downloader

A simple downloader library with support for caching. It is designed to be used with applications requiring the downloaded assets to be cached in file system.

# Features

- Async
- Cache downloaded file in a directory in filesystem.
- Check if a file is available in cache.
- Uses SHA256 for verifying cached files.
- Optional support to download files without caching.

# Sample Usage

```rust
#[tokio::main]
async fn main() {
    let downloader = bb_downloader::Downloader::new("/tmp").unwrap();

    let sha = [0u8; 32];
    let url = "https://example.com/img.jpg";

    // Download with just URL
    downloader.download(url, None).await.unwrap();

    // Check if the file is in cache
    assert!(downloader.check_cache_from_url(url).is_some());

    // Will fetch directly from cache instead of re-downloading
    downloader.download(url, None).await.unwrap();

    // Since it was cached by URL, will fail with SHA256.
    assert!(!downloader.check_cache_from_sha(sha).is_some());

    // Will re-download the file
    downloader.download_with_sha(url, sha, None).await.unwrap();

    assert!(downloader.check_cache_from_sha(sha).is_some());
}
```
