use std::path::Path;

/// A trait to signify Os Images. Flashers in this crate can take any file as an input that
/// implements this trait.
pub trait Resolvable {
    type ResolvedType;

    /// Get the local path to an image. Network calls can be done here.
    fn resolve(
        &self,
        join_set: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> impl Future<Output = std::io::Result<Self::ResolvedType>>;
}

#[derive(Debug, Clone)]
pub struct LocalStringFile(Box<Path>);

impl LocalStringFile {
    /// Construct a new local image from path.
    pub const fn new(path: Box<Path>) -> Self {
        Self(path)
    }
}

impl Resolvable for LocalStringFile {
    type ResolvedType = Box<str>;

    async fn resolve(
        &self,
        _: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> std::io::Result<Self::ResolvedType> {
        tokio::fs::read_to_string(&self.0).await.map(Into::into)
    }
}

#[derive(Debug, Clone)]
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
fn size(file: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    file.size()
}

#[cfg(windows)]
fn size(file: &std::fs::Metadata) -> u64 {
    use std::os::windows::fs::MetadataExt;
    file.file_size()
}
