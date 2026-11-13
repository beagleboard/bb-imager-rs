use std::borrow::Cow;
use std::sync::Arc;

use bb_config::config;
use url::Url;

#[derive(Debug, Clone)]
pub struct Board {
    pub id: i64,
    pub name: Box<str>,
    pub icon: Option<Arc<Url>>,
    pub description: Arc<str>,
    pub documentation: Option<Url>,
    pub specification: Vec<(Box<str>, Box<str>)>,
    pub oshw: Option<Box<str>>,
    pub flasher: config::Flasher,
    pub instructions: Option<Box<str>>,
    pub bootfs: Option<config::Bootfs>,
}

impl Board {
    pub(crate) fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let spec: Vec<u8> = value.get("specification")?;
        let bootfs: Option<Vec<u8>> = value.get("bootfs")?;

        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            icon: value.get::<_, Option<Url>>("icon")?.map(Arc::new),
            description: value.get("description")?,
            documentation: value.get("documentation")?,
            specification: serde_json::from_slice(&spec).unwrap(),
            oshw: value.get("oshw")?,
            flasher: value.get("flasher")?,
            instructions: value.get("instructions")?,
            bootfs: bootfs.map(|x| serde_json::from_slice(&x).unwrap()),
        })
    }
}

/// One row of the board list.
#[derive(Default, Debug, Clone)]
pub struct BoardItem {
    pub id: i64,
    pub icon: Option<Arc<url::Url>>,
    pub name: Box<str>,
}

impl BoardItem {
    pub(crate) fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            icon: value.get::<_, Option<Url>>("icon")?.map(Arc::new),
            name: value.get("name")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OsImage {
    pub id: i64,
    pub name: Box<str>,
    pub description: Box<str>,
    pub icon: Arc<Url>,
    pub url: Box<Url>,
    pub image_download_size: i64,
    pub image_download_sha256: [u8; 32],
    pub extract_size: i64,
    pub release_date: chrono::NaiveDate,
    pub init_format: bb_config::config::InitFormat,
    pub bmap: Option<Box<Url>>,
    pub sbom: Option<Url>,
    pub info_text: Option<Arc<str>>,
    pub support: Option<Url>,
}

impl OsImage {
    pub(crate) fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            icon: Arc::new(value.get("icon")?),
            description: value.get("description")?,
            url: Box::new(value.get("url")?),
            image_download_size: value.get("image_download_size")?,
            image_download_sha256: value.get("image_download_sha256")?,
            extract_size: value.get("extract_size")?,
            release_date: value.get("release_date")?,
            init_format: value.get("init_format")?,
            bmap: value.get::<_, Option<Url>>("bmap")?.map(Box::new),
            sbom: value.get::<_, Option<Url>>("sbom")?,
            info_text: value.get("info_text")?,
            support: value.get("support")?,
        })
    }
}

/// Identifies one row of the image list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageId {
    OsImage(i64),
    OsSublist((i64, config::Flasher)),
}

impl ImageId {
    /// Row from `os_images`: reads `id`.
    pub(crate) fn os_image_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self::OsImage(row.get("id")?))
    }

    /// Row from `os_sublists`: reads `id` and `flasher`.
    pub(crate) fn os_sublist_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self::OsSublist((row.get("id")?, row.get("flasher")?)))
    }
}

/// One row of the image list.
#[derive(Debug, Clone)]
pub struct ImageItem {
    pub id: ImageId,
    pub icon: Option<Arc<Url>>,
    pub label: Cow<'static, str>,
}

impl ImageItem {
    pub(crate) fn from_row(
        row: &rusqlite::Row<'_>,
        id: fn(&rusqlite::Row<'_>) -> rusqlite::Result<ImageId>,
    ) -> rusqlite::Result<Self> {
        Ok(Self {
            id: id(row)?,
            icon: Some(Arc::new(row.get("icon")?)),
            label: row.get::<_, String>("name")?.into(),
        })
    }
}
