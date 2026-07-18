//! Integration tests for the `Bmap` builder API and `from_xml` error paths.
//!
//! `tests/parse.rs` already covers the happy-path `from_xml` against a real
//! fixture. These tests cover the public `builder()` path (untested before),
//! the `BmapBuilderError` matrix, and the `XmlError` failure branches.

use bb_bmap_parser::{Bmap, BmapBuilderError, HashType, HashValue, XmlError};

/// Build a fully-specified `Bmap`, exercising every getter, the byte-range
/// clamping in `add_block_range`, and `total_mapped_size`.
#[test]
fn builder_happy_path() {
    let mut builder = Bmap::builder();
    builder
        .image_size(10_000)
        .block_size(4096)
        .blocks(3)
        .mapped_blocks(2)
        .checksum_type(HashType::Sha256);
    builder.add_block_range(0, 0, HashValue::Sha256([1; 32]));
    // start=2 -> offset 8192; a full block would run to 12288 > image_size,
    // so the length must be clamped to 10_000 - 8192 = 1808.
    builder.add_block_range(2, 2, HashValue::Sha256([2; 32]));

    let bmap = builder.build().expect("all required fields set");

    assert_eq!(bmap.image_size(), 10_000);
    assert_eq!(bmap.block_size(), 4096);
    assert_eq!(bmap.blocks(), 3);
    assert_eq!(bmap.mapped_blocks(), 2);
    assert_eq!(bmap.checksum_type(), HashType::Sha256);
    assert_eq!(bmap.total_mapped_size(), 4096 * 2);

    // `block_map()` is an ExactSizeIterator.
    assert_eq!(bmap.block_map().len(), 2);
    let ranges: Vec<_> = bmap.block_map().collect();

    assert_eq!(ranges[0].offset(), 0);
    assert_eq!(ranges[0].length(), 4096);
    assert_eq!(ranges[0].checksum(), HashValue::Sha256([1; 32]));
    assert_eq!(ranges[0].checksum().as_slice(), &[1u8; 32]);

    assert_eq!(ranges[1].offset(), 8192);
    assert_eq!(ranges[1].length(), 1808, "final range should be clamped");
}

/// Each omitted required field yields the matching `BmapBuilderError`. `build()`
/// checks fields in declaration order (image_size, block_size, blocks,
/// mapped_blocks, checksum_type), so omitting one reliably surfaces its error.
#[test]
fn builder_missing_field_matrix() {
    // (label, configure-all-but-one, expected error)
    let cases: Vec<(&str, Box<dyn Fn(&mut bb_bmap_parser::BmapBuilder)>, BmapBuilderError)> = vec![
        (
            "image_size",
            Box::new(|b| {
                b.block_size(4096).blocks(1).mapped_blocks(1).checksum_type(HashType::Sha256);
            }),
            BmapBuilderError::MissingImageSize,
        ),
        (
            "block_size",
            Box::new(|b| {
                b.image_size(4096).blocks(1).mapped_blocks(1).checksum_type(HashType::Sha256);
            }),
            BmapBuilderError::MissingBlockSize,
        ),
        (
            "blocks",
            Box::new(|b| {
                b.image_size(4096).block_size(4096).mapped_blocks(1).checksum_type(HashType::Sha256);
            }),
            BmapBuilderError::MissingBlocks,
        ),
        (
            "mapped_blocks",
            Box::new(|b| {
                b.image_size(4096).block_size(4096).blocks(1).checksum_type(HashType::Sha256);
            }),
            BmapBuilderError::MissingMappedBlocks,
        ),
        (
            "checksum_type",
            Box::new(|b| {
                b.image_size(4096).block_size(4096).blocks(1).mapped_blocks(1);
            }),
            BmapBuilderError::MissingChecksumType,
        ),
    ];

    for (label, configure, expected) in cases {
        let mut builder = Bmap::builder();
        configure(&mut builder);
        let err = builder.build().expect_err(label);
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&expected),
            "missing {label} should yield {expected:?}, got {err:?}",
        );
    }
    // NOTE: `BmapBuilderError::NoBlockRanges` is defined but never produced by
    // `build()` (it does not validate that the block map is non-empty), so it
    // cannot be exercised through the public API.
}

fn minimal_xml(checksum_type: &str, chksum: &str, range: &str) -> String {
    format!(
        r#"<?xml version="1.0" ?>
<bmap version="2.0">
  <ImageSize>8192</ImageSize>
  <BlockSize>4096</BlockSize>
  <BlocksCount>2</BlocksCount>
  <MappedBlocksCount>1</MappedBlocksCount>
  <ChecksumType>{checksum_type}</ChecksumType>
  <BmapFileChecksum>0000000000000000000000000000000000000000000000000000000000000000</BmapFileChecksum>
  <BlockMap>
    <Range chksum="{chksum}">{range}</Range>
  </BlockMap>
</bmap>"#
    )
}

const VALID_SHA: &str = "5feceb66ffc86f38d952786c6d696c79c2dbc239dd4e91b46729d73a27fb57e9";

/// A single-block range ("0", no `-`) exercises the `end = start` branch.
#[test]
fn from_xml_single_block_range() {
    let xml = minimal_xml("sha256", VALID_SHA, "0");
    let bmap = Bmap::from_xml(&xml).expect("valid minimal bmap");
    assert_eq!(bmap.block_map().len(), 1);
    let range = bmap.block_map().next().unwrap();
    assert_eq!(range.offset(), 0);
    assert_eq!(range.length(), 4096);
}

#[test]
fn from_xml_rejects_malformed_xml() {
    let err = Bmap::from_xml("this is not xml <<<").expect_err("garbage input");
    assert!(matches!(err, XmlError::XmlParsError(_)), "got {err:?}");
}

#[test]
fn from_xml_rejects_wrong_length_checksum() {
    // 4 hex chars where 64 are required.
    let xml = minimal_xml("sha256", "abcd", "0");
    let err = Bmap::from_xml(&xml).expect_err("short checksum");
    assert!(matches!(err, XmlError::InvalidChecksum(_)), "got {err:?}");
}

#[test]
fn from_xml_rejects_non_hex_checksum() {
    // Correct length (64) but contains non-hex characters.
    let bad = format!("zz{}", &VALID_SHA[2..]);
    let xml = minimal_xml("sha256", &bad, "0");
    let err = Bmap::from_xml(&xml).expect_err("non-hex checksum");
    assert!(matches!(err, XmlError::InvalidChecksum(_)), "got {err:?}");
}

#[test]
fn from_xml_rejects_unknown_checksum_type() {
    let xml = minimal_xml("md5", VALID_SHA, "0");
    let err = Bmap::from_xml(&xml).expect_err("unknown checksum type");
    // The unknown type fails during deserialization, so it surfaces as a parse
    // error rather than `XmlError::UnknownChecksumType` (which is unused).
    assert!(matches!(err, XmlError::XmlParsError(_)), "got {err:?}");
}
