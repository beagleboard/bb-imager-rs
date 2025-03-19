//! # Introduction
//!
//! BeagleBoard.org maintains a json file with the list of all board images which can be used by
//! applications (like BeagleBoard Imaging Utility) to get a list of latest images for each board.
//!
//! This crate provides abstractions to parse and generate distros.json file.
//!
//! # Usage
//!
//! ```no_run
//! let config: bb_config::Config = reqwest::blocking::get(bb_config::DISTROS_URL)
//!     .unwrap()
//!     .json()
//!     .unwrap();
//!
//! // Convert back to JSON
//! let json_config = serde_json::to_string_pretty(&config).unwrap();
//! ```

pub mod config;

/// URL for the BeagleBoard.org `distros.json` file
pub const DISTROS_URL: &str = "https://www.beagleboard.org/distros.json";

pub use config::Config;

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        let data = include_bytes!("../../config.json");
        serde_json::from_slice::<super::Config>(data).unwrap();
    }
}
