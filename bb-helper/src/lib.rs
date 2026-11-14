//! Helper utilities shared across the BeagleBoard.org imaging tools.
//!
//! All functionality is gated behind feature flags:
//!
//! - `cancel`: [`cancel::CancellationToken`] for cooperative cancellation.
//! - `reader_progress`: [`reader_progress::ReaderWithProgress`] for reporting read progress.
//! - `file_stream`: [`file_stream::file_stream`] for a file-backed stream with async write and
//!   sync read halves.
//! - `status`: [`status::Status`], the progress reported by the flashers.

#[cfg(feature = "cancel")]
pub mod cancel;
#[cfg(feature = "file_stream")]
pub mod file_stream;
#[cfg(feature = "fmt")]
pub mod fmt;
#[cfg(feature = "reader_progress")]
pub mod reader_progress;
#[cfg(feature = "status")]
pub mod status;
