use std::path::Path;

#[derive(Debug, Clone)]
/// A resolvable that reads a local file as a string.
pub struct LocalStringFile(Box<Path>);

impl LocalStringFile {
    /// Construct a new local image from path.
    pub(crate) const fn new(path: Box<Path>) -> Self {
        Self(path)
    }

    pub(crate) fn into_fn(self) -> impl FnOnce() -> std::io::Result<Box<str>> {
        move || std::fs::read_to_string(self.0).map(Into::into)
    }
}
