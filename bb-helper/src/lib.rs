//! Helper utilities for the BeagleBoard imager.
//!
//! This crate provides common functionality used across the imager components,
//! including file streaming and resolvable image types.

#[cfg(feature = "file_stream")]
pub mod file_stream;
#[cfg(feature = "resolvable")]
pub mod resolvable;
