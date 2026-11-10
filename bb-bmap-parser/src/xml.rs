use crate::bmap::{BmapBuilder, BmapBuilderError, HashValue};
use quick_xml::de::{DeError, from_str};
use serde::Deserialize;
use thiserror::Error;

/// Custom deserializer to first trim whitespace around text elements before converting.
/// Should be unnecessary once https://github.com/tafia/quick-xml/issues/900 gets fixed
fn deserialize_trimmed<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let s = <&str>::deserialize(deserializer)?;
    s.trim().parse().map_err(serde::de::Error::custom)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HashType {
    Sha256,
}

impl std::str::FromStr for HashType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(Self::Sha256),
            _ => Err("Unsupported"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Range {
    #[serde(rename = "@chksum", deserialize_with = "deserialize_trimmed")]
    chksum: String,
    #[serde(rename = "$value", deserialize_with = "deserialize_trimmed")]
    range: String,
}

#[derive(Debug, Deserialize)]
struct BlockMap {
    #[serde(rename = "Range")]
    ranges: Vec<Range>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Bmap {
    #[serde(rename = "@version", deserialize_with = "deserialize_trimmed")]
    version: String,
    #[serde(rename = "ImageSize", deserialize_with = "deserialize_trimmed")]
    image_size: u64,
    #[serde(rename = "BlockSize", deserialize_with = "deserialize_trimmed")]
    block_size: u64,
    #[serde(rename = "BlocksCount", deserialize_with = "deserialize_trimmed")]
    blocks_count: u64,
    #[serde(rename = "MappedBlocksCount", deserialize_with = "deserialize_trimmed")]
    mapped_blocks_count: u64,
    #[serde(rename = "ChecksumType", deserialize_with = "deserialize_trimmed")]
    checksum_type: HashType,
    #[serde(rename = "BmapFileChecksum", deserialize_with = "deserialize_trimmed")]
    bmap_file_checksum: String,
    #[serde(rename = "BlockMap")]
    block_map: BlockMap,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum XmlError {
    #[error("Failed to parse bmap XML: {0}")]
    XmlParseError(#[from] DeError),
    #[error("Invalid bmap file: {0}")]
    InvalidFileError(#[from] BmapBuilderError),
    #[error("Invalid checksum: {0}")]
    InvalidChecksum(String),
    #[error("Invalid block size: {0}")]
    InvalidBlockSize(u64),
    #[error("Invalid block range: {0}")]
    InvalidRange(String),
}

const fn hexdigit_to_u8(c: u8) -> Option<u8> {
    match c {
        b'a'..=b'f' => Some(c - b'a' + 0xa),
        b'A'..=b'F' => Some(c - b'A' + 0xa),
        b'0'..=b'9' => Some(c - b'0'),
        _ => None,
    }
}

fn str_to_digest(s: String, digest: &mut [u8]) -> Result<(), XmlError> {
    let l = digest.len();
    if s.len() != l * 2 {
        return Err(XmlError::InvalidChecksum(format!(
            "No enough chars: {} {}",
            s,
            s.len()
        )));
    }

    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = match hexdigit_to_u8(chunk[0]) {
            Some(v) => v,
            None => return Err(XmlError::InvalidChecksum(s)),
        };
        let lo = match hexdigit_to_u8(chunk[1]) {
            Some(v) => v,
            None => return Err(XmlError::InvalidChecksum(s)),
        };
        digest[i] = hi << 4 | lo;
    }

    Ok(())
}

/// Parses `start-end` or `start` (in blocks), checking it lies inside the image
/// so that `BmapBuilder::add_block_range` cannot overflow.
fn parse_range(range: &str, block_size: u64, image_size: u64) -> Result<(u64, u64), XmlError> {
    let invalid = || XmlError::InvalidRange(range.into());
    let (start, end) = range.split_once('-').unwrap_or((range, range));
    let start: u64 = start.parse().map_err(|_| invalid())?;
    let end: u64 = end.parse().map_err(|_| invalid())?;

    let starts_in_image = start
        .checked_mul(block_size)
        .is_some_and(|offset| offset < image_size);
    let length_fits = end
        .checked_sub(start)
        .and_then(|x| x.checked_add(1))
        .and_then(|x| x.checked_mul(block_size))
        .is_some();

    if starts_in_image && length_fits {
        Ok((start, end))
    } else {
        Err(invalid())
    }
}

pub(crate) fn from_xml(xml: &str) -> Result<crate::bmap::Bmap, XmlError> {
    let b: Bmap = from_str(xml)?;
    if b.block_size == 0 {
        return Err(XmlError::InvalidBlockSize(b.block_size));
    }
    let mut builder = BmapBuilder::default();
    let hash_type = b.checksum_type;
    builder
        .image_size(b.image_size)
        .block_size(b.block_size)
        .blocks(b.blocks_count)
        .checksum_type(hash_type)
        .mapped_blocks(b.mapped_blocks_count);

    for range in b.block_map.ranges {
        let (start, end) = parse_range(&range.range, b.block_size, b.image_size)?;

        let checksum = match hash_type {
            HashType::Sha256 => {
                let mut v = [0; 32];
                str_to_digest(range.chksum, &mut v)?;
                HashValue::Sha256(v)
            }
        };
        builder.add_block_range(start, end, checksum);
    }

    builder.build().map_err(std::convert::Into::into)
}
