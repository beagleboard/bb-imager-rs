use std::path::Path;

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
