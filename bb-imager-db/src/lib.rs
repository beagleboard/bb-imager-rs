//! SQLite-backed cache of the BeagleBoard board and OS image catalog
//! (`config.json` plus any remote configs it references).

mod db;
mod types;

pub use db::Db;
pub use rusqlite::{Error, Result};
pub use types::{Board, BoardItem, ImageId, ImageItem, OsImage};
