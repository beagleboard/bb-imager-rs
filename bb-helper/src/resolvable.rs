use std::path::Path;

/// A trait for resolvable image sources.
///
/// Types implementing this trait can resolve themselves to local file representations
/// that flashers can use. This allows for various image sources like local files,
/// remote URLs, or generated content.
pub trait Resolvable {
    type ResolvedType;

    /// Resolves the image source to a local representation.
    ///
    /// This method may perform network requests or other I/O operations.
    /// The `join_set` can be used to spawn background tasks if needed.
    fn resolve(
        &self,
        join_set: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> impl Future<Output = std::io::Result<Self::ResolvedType>>;
}

#[derive(Debug, Clone)]
/// A resolvable that reads a local file as a string.
pub struct LocalStringFile(Box<Path>);

impl LocalStringFile {
    /// Construct a new local image from path.
    pub const fn new(path: Box<Path>) -> Self {
        Self(path)
    }
}

impl IntoFuture for LocalStringFile {
    type Output = std::io::Result<Box<str>>;
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { tokio::fs::read_to_string(&self.0).await.map(Into::into) })
    }
}

#[derive(Debug, Clone)]
/// A resolvable that opens a local file and returns its handle with size.
pub struct LocalFile(Box<Path>);

impl LocalFile {
    /// Construct a new local image from path.
    pub const fn new(path: Box<Path>) -> Self {
        Self(path)
    }
}

impl Resolvable for LocalFile {
    type ResolvedType = (std::fs::File, u64);

    async fn resolve(
        &self,
        _: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> std::io::Result<Self::ResolvedType> {
        let f = tokio::fs::File::open(&self.0).await?.into_std().await;
        let size = size(&f.metadata()?);
        Ok((f, size))
    }
}

#[cfg(unix)]
/// Gets the file size from metadata on Unix systems.
fn size(file: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    file.size()
}

#[cfg(windows)]
/// Gets the file size from metadata on Windows systems.
fn size(file: &std::fs::Metadata) -> u64 {
    use std::os::windows::fs::MetadataExt;
    file.file_size()
}
