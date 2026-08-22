//! This module handles interaction with sqlite db used for config.

use std::sync::{Arc, Mutex};

use bb_config::config;
use rusqlite::Connection;
use url::Url;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct Db {
    _f: Arc<tempfile::NamedTempFile>,
    db: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Board {
    pub(crate) id: i64,
    pub(crate) name: Box<str>,
    pub(crate) flasher: config::Flasher,
    pub(crate) instructions: Option<Box<str>>,
}

impl Board {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            flasher: value.get("flasher")?,
            instructions: value.get("instructions")?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OsImage {
    pub(crate) id: i64,
    pub(crate) name: Box<str>,
    // Boxed because `Url` is 80 bytes inline, and both end up behind a `Box` in
    // `helpers::RemoteImage`/`helpers::Bmap` anyway. This struct rides along in
    // `BBImagerMessage::SelectRemoteOs`, which sizes the whole message enum.
    pub(crate) url: Box<Url>,
    pub(crate) image_download_sha256: [u8; 32],
    pub(crate) extract_size: i64,
    pub(crate) init_format: bb_config::config::InitFormat,
    pub(crate) bmap: Option<Box<Url>>,
    pub(crate) info_text: Option<std::sync::Arc<str>>,
}

impl OsImage {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            init_format: value.get("init_format")?,
            info_text: value.get("info_text")?,
            url: Box::new(value.get("url")?),
            image_download_sha256: value.get("image_download_sha256")?,
            extract_size: value.get("extract_size")?,
            bmap: value.get::<_, Option<Url>>("bmap")?.map(Box::new),
        })
    }
}

const MIGRATIONS: &str = include_str!("../../migrations/20260316134019_init.sql");

impl Db {
    pub(crate) fn new() -> rusqlite::Result<Self> {
        let f = tempfile::NamedTempFile::new().unwrap();
        tracing::info!("DB Path: {}", f.path().display());
        let db = Connection::open(f.path())?;

        Ok(Self {
            _f: Arc::new(f),
            db: Arc::new(Mutex::new(db)),
        })
    }

    pub(crate) fn init(&self) -> rusqlite::Result<()> {
        // Populate initial data
        let cfg =
            serde_json::from_slice::<bb_config::config::Config>(crate::constants::DEFAULT_CONFIG)
                .expect("Failed to parse config");

        let mut db = self.db.lock().unwrap();

        // Run migrations
        db.execute_batch(MIGRATIONS)?;

        Self::add_config_internal(&mut db, cfg, None)
    }

    pub(crate) fn add_config(
        &self,
        cfg: config::Config,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<()> {
        let mut db = self.db.lock().unwrap();
        Self::add_config_internal(&mut db, cfg, remote_config_id)
    }

    fn add_config_internal(
        db: &mut Connection,
        cfg: config::Config,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<()> {
        let tx = db.transaction()?;

        if let Some(x) = remote_config_id {
            Self::remote_config_fetched(&tx, x)?;
        }

        Self::insert_remote_config(&tx, cfg.imager.remote_configs.iter())?;

        for dev in cfg
            .imager
            .devices
            .iter()
            .filter(|x| crate::helpers::flasher_supported(x.flasher))
        {
            Self::insert_board(&tx, dev)?;
        }

        Self::insert_os_list_items(&tx, &cfg.os_list, None, remote_config_id)?;

        tx.commit()
    }

    fn insert_remote_config<'a>(
        exec: &Connection,
        remote_configs: impl Iterator<Item = &'a Url>,
    ) -> rusqlite::Result<()> {
        let mut stmt = exec.prepare_cached(
            r#"
                INSERT INTO remote_configs(url) VALUES ($1) 
                ON CONFLICT DO NOTHING
                "#,
        )?;
        for u in remote_configs {
            stmt.execute([u])?;
        }

        Ok(())
    }

    fn remote_config_fetched(exec: &Connection, id: i64) -> rusqlite::Result<()> {
        let mut stmt =
            exec.prepare_cached("UPDATE remote_configs SET fetched = TRUE WHERE id = $1")?;
        stmt.execute([id])?;
        Ok(())
    }

    // Not cached: runs exactly once per process, from the `DbInitSuccess` handler.
    pub(crate) fn remote_configs(&self) -> rusqlite::Result<Vec<(i64, Url)>> {
        let db = self.db.lock().unwrap();

        let res = db
            .prepare("SELECT id, url FROM remote_configs WHERE fetched = FALSE")?
            .query_map([], |r| {
                let id: i64 = r.get("id")?;
                let u: Url = r.get("url")?;

                Ok((id, u))
            })?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn os_remote_sublist_resolve(
        &self,
        id: i64,
        subitems: &[bb_config::config::OsListItem],
    ) -> rusqlite::Result<()> {
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;

        {
            let mut stmt =
                tx.prepare_cached("UPDATE os_sublists SET subitems_url = NULL WHERE id = $1")?;
            stmt.execute([id])?;
        }

        Self::insert_os_list_items(&tx, subitems, Some(id), None)?;

        tx.commit()
    }

    fn insert_os_list_items(
        exec: &Connection,
        items: &[config::OsListItem],
        start_pid: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<()> {
        let mut imgs = Vec::from_iter(items.iter().map(|x| (start_pid, x)));

        while let Some((pid, img)) = imgs.pop() {
            match img {
                config::OsListItem::Image(os_image) => {
                    let id = Self::insert_image(exec, os_image, pid, remote_config_id)?;
                    if let Some(p) = pid {
                        Self::insert_sublist_boards(exec, p, id)?
                    }
                }
                config::OsListItem::SubList(os_sub_list) => {
                    if crate::helpers::flasher_supported(os_sub_list.flasher) {
                        let id = Self::insert_sub_list(exec, os_sub_list, pid, remote_config_id)?;
                        imgs.extend(os_sub_list.subitems.iter().map(|x| (Some(id), x)));
                    }
                }
                config::OsListItem::RemoteSubList(os_remote_sub_list) => {
                    if crate::helpers::flasher_supported(os_remote_sub_list.flasher) {
                        let id = Self::insert_remote_image(
                            exec,
                            os_remote_sub_list,
                            pid,
                            remote_config_id,
                        )?;
                        Self::insert_remote_sublist_boards(exec, id)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn insert_remote_sublist_boards(exec: &Connection, sublist_id: i64) -> rusqlite::Result<()> {
        let mut stmt = exec.prepare_cached(
            r#"
            WITH RECURSIVE parents(id) AS (
                SELECT $1

                UNION ALL

                SELECT s.parent_id
                FROM os_sublists s
                JOIN parents p ON s.id = p.id
                WHERE s.parent_id IS NOT NULL
            )
            INSERT OR IGNORE INTO os_sublist_boards(sublist_id, board_id)
            SELECT p.id, osb.board_id
            FROM parents p
            JOIN os_sublist_boards osb ON osb.sublist_id = $1;
            "#,
        )?;
        stmt.execute([sublist_id])?;
        Ok(())
    }

    fn insert_sublist_boards(
        exec: &Connection,
        parent_id: i64,
        image_id: i64,
    ) -> rusqlite::Result<()> {
        let mut stmt = exec.prepare_cached(
            r#"
            WITH RECURSIVE ancestors(id) AS (
                SELECT $1
                UNION ALL
                SELECT parent_id
                FROM os_sublists
                JOIN ancestors ON os_sublists.id = ancestors.id
                WHERE parent_id IS NOT NULL
            )
            INSERT OR IGNORE INTO os_sublist_boards (sublist_id, board_id)
            SELECT ancestors.id, ib.board_id
            FROM ancestors
            JOIN os_image_boards ib ON ib.image_id = $2
            "#,
        )?;
        stmt.execute([parent_id, image_id])?;
        Ok(())
    }

    fn insert_sub_list(
        exec: &Connection,
        item: &config::OsSubList,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let mut stmt = exec.prepare_cached(
            r#"
             INSERT INTO os_sublists(parent_id, name, description, icon, flasher, remote_config_id)
             VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )?;
        stmt.insert(rusqlite::params![
            parent_id,
            item.name,
            item.description,
            item.icon,
            item.flasher,
            remote_config_id,
        ])
    }

    fn insert_board(exec: &Connection, board: &config::Device) -> rusqlite::Result<()> {
        let spec = serde_json::to_vec(&board.specification).unwrap();

        // Insert or update board
        let mut stmt = exec.prepare_cached(
            r#"
        INSERT INTO boards(
            name,
            description,
            icon,
            flasher,
            instructions,
            oshw,
            specification,
            documentation
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            icon = excluded.icon,
            flasher = excluded.flasher,
            instructions = excluded.instructions,
            oshw = excluded.oshw,
            specification = excluded.specification,
            documentation = excluded.documentation
        RETURNING id
        "#,
        )?;
        let id: i64 = stmt.query_row(
            rusqlite::params![
                board.name,
                board.description,
                board.icon,
                board.flasher,
                board.instructions,
                board.oshw,
                spec,
                board.documentation
            ],
            |r| r.get(0),
        )?;

        // Remove old tags
        exec.execute(
            r#"
        DELETE FROM board_tags
        WHERE board_id = $1
        "#,
            [id],
        )?;

        // Insert new tags. `tags` is a plain slice, so a config that repeats a tag
        // would otherwise trip the (board_id, tag) primary key.
        let mut stmt = exec.prepare_cached(
            r#"
            INSERT OR IGNORE INTO board_tags(board_id, tag)
            VALUES ($1, $2)
            "#,
        )?;
        for tag in &board.tags {
            stmt.execute(rusqlite::params![id, tag.as_ref()])?;
        }

        Ok(())
    }

    fn insert_image(
        exec: &Connection,
        img: &config::OsImage,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let mut stmt = exec.prepare_cached(
            r#"
            INSERT INTO os_images(name, parent_id, description, icon, url,
                image_download_size, image_download_sha256, extract_size,
                release_date, init_format, bmap, info_text, remote_config_id, support)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )?;
        let id = stmt.insert(rusqlite::params![
            img.name,
            parent_id,
            img.description,
            img.icon,
            img.url,
            img.image_download_size.map(|x| i64::try_from(x).unwrap()),
            img.image_download_sha256,
            i64::try_from(img.extract_size).unwrap(),
            img.release_date,
            img.init_format,
            img.bmap,
            img.info_text,
            remote_config_id,
            img.support
        ])?;

        // OR IGNORE: two distinct tags can resolve to the same board, and `devices`
        // may repeat a tag, either of which collides on (image_id, board_id).
        let mut stmt = exec.prepare_cached(
            r#"
            INSERT OR IGNORE INTO os_image_boards(image_id, board_id)
            SELECT $1, b.board_id
            FROM board_tags b
            WHERE b.tag = $2
                "#,
        )?;
        for dev in &img.devices {
            stmt.execute(rusqlite::params![id, dev.as_ref()])?;
        }

        Ok(id)
    }

    fn insert_remote_image(
        exec: &Connection,
        img: &config::OsRemoteSubList,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let mut stmt = exec.prepare_cached(
            r#"
            INSERT INTO os_sublists(parent_id, name, description, icon, 
                flasher, subitems_url, remote_config_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )?;
        let id = stmt.insert(rusqlite::params![
            parent_id,
            img.name,
            img.description,
            img.icon,
            img.flasher,
            img.subitems_url,
            remote_config_id
        ])?;

        let mut stmt = exec.prepare_cached(
            r#"
            INSERT OR IGNORE INTO os_sublist_boards(sublist_id, board_id)
            SELECT $1, b.board_id
            FROM board_tags b
            WHERE b.tag = $2
                "#,
        )?;
        for dev in &img.devices {
            stmt.execute(rusqlite::params![id, dev.as_ref()])?;
        }

        Ok(id)
    }

    /// Get all board icons.
    pub(crate) fn board_icons(&self) -> rusqlite::Result<Vec<Arc<Url>>> {
        let db = self.db.lock().unwrap();
        let mut stmt =
            db.prepare_cached("SELECT DISTINCT icon FROM boards WHERE icon IS NOT NULL")?;
        let res = stmt
            .query_map([], |r| r.get(0).map(Arc::new))?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    /// Get board list data. (ID, Icon, Name)
    pub(crate) fn board_list(
        &self,
        search: &str,
    ) -> rusqlite::Result<Box<[bb_imager_ui::board_selection::Board]>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            "SELECT id, icon, name FROM boards WHERE name LIKE $1 COLLATE NOCASE",
        )?;
        let res = stmt
            .query_map([format!("%{}%", search)], |value| {
                Ok(bb_imager_ui::board_selection::Board {
                    id: value.get("id")?,
                    icon: value.get::<_, Option<Url>>("icon")?.map(Arc::new),
                    name: value.get("name")?,
                })
            })?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn board_by_id(&self, id: i64) -> rusqlite::Result<Board> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
        SELECT id, name, flasher, instructions
        FROM boards
        WHERE id = $1"#,
        )?;
        stmt.query_row([id], Board::from_row)
    }

    pub(crate) fn os_image_by_id(&self, id: i64) -> rusqlite::Result<OsImage> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT id, name, url, image_download_sha256, extract_size, init_format, bmap, info_text
            FROM os_images WHERE id = $1"#,
        )?;
        stmt.query_row([id], OsImage::from_row)
    }

    pub(crate) fn os_image_items(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<bb_imager_ui::img_selection::ImageItem>> {
        let a = self.os_images_by_board_id(board_id, parent_id)?;
        let b = self.os_sublists(board_id, parent_id)?;

        Ok(a.into_iter().chain(b).collect())
    }

    fn os_images_by_board_id(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<bb_imager_ui::img_selection::ImageItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT oi.id, oi.name, oi.icon, oi.description, oi.release_date, oi.extract_size
            FROM os_images oi
            JOIN os_image_boards oib ON oi.id = oib.image_id
            WHERE oib.board_id = $1 
                AND (
                        ($2 IS NULL AND oi.parent_id IS NULL) 
                        OR oi.parent_id = $2
                )
            ORDER BY oi.remote_config_id NULLS LAST"#,
        )?;
        let res = stmt
            .query_map(rusqlite::params![board_id, parent_id], |value| {
                let size: i64 = value.get("extract_size")?;
                Ok(bb_imager_ui::img_selection::ImageItem {
                    id: bb_imager_ui::img_selection::ImageId::OsImage(value.get("id")?),
                    label: value.get("name")?,
                    icon: value.get::<_, Option<Url>>("icon")?.map(Arc::new),
                    description: value.get("description")?,
                    size: Some(size as u64),
                    release_date: value.get("release_date")?,
                })
            })?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    fn os_sublists(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<bb_imager_ui::img_selection::ImageItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT s.id, s.name, s.icon, s.flasher, s.description
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1
              AND (
                    ($2 IS NULL AND s.parent_id IS NULL)
                 OR s.parent_id = $2
              )
            ORDER BY s.remote_config_id NULLS LAST"#,
        )?;
        let res = stmt
            .query_map(rusqlite::params![board_id, parent_id], |value| {
                Ok(bb_imager_ui::img_selection::ImageItem {
                    id: bb_imager_ui::img_selection::ImageId::OsSublist((
                        value.get("id")?,
                        value.get("flasher")?,
                    )),
                    label: value.get("name")?,
                    icon: value.get::<_, Option<Url>>("icon")?.map(Arc::new),
                    description: value.get("description")?,
                    size: None,
                    release_date: None,
                })
            })?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn os_remote_sublists(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<(i64, Url)>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT s.id, s.subitems_url
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1
                AND s.subitems_url IS NOT NULL
                AND (
                    ($2 IS NULL AND s.parent_id IS NULL)
                    OR s.parent_id = $2
                )"#,
        )?;
        let res = stmt
            .query_map(rusqlite::params![board_id, parent_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn os_sublist_parent(&self, sublist_id: i64) -> rusqlite::Result<Option<i64>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached("SELECT parent_id FROM os_sublists WHERE id = $1")?;
        stmt.query_row([sublist_id], |r| r.get(0))
    }

    pub(crate) fn os_image_icons_by_board_id(
        &self,
        board_id: i64,
    ) -> rusqlite::Result<Vec<Arc<Url>>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT oi.icon FROM os_images oi 
            JOIN os_image_boards oib ON oi.id = oib.image_id 
            WHERE oib.board_id = $1

            UNION

            SELECT s.icon
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1"#,
        )?;
        let res = stmt
            .query_map([board_id], |r| r.get(0).map(Arc::new))?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn os_remote_sublists_by_remote_config(
        &self,
        board_id: i64,
        remote_config_id: i64,
    ) -> rusqlite::Result<Vec<(i64, Url)>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT s.id, s.subitems_url
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1 
                AND s.subitems_url IS NOT NULL 
                AND s.remote_config_id = $2"#,
        )?;
        let res = stmt
            .query_map([board_id, remote_config_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn os_remote_sublists_by_board(
        &self,
        board_id: i64,
    ) -> rusqlite::Result<Vec<(i64, Url)>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT s.id, s.subitems_url
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1 AND s.subitems_url IS NOT NULL"#,
        )?;
        let res = stmt
            .query_map([board_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn os_images_by_name(
        &self,
        board_id: i64,
        search: &str,
    ) -> rusqlite::Result<Vec<bb_imager_ui::img_selection::ImageItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT oi.id, oi.name, oi.icon, oi.description, oi.release_date, oi.extract_size
            FROM os_images oi
            JOIN os_image_boards oib ON oi.id = oib.image_id
            WHERE oib.board_id = $1 AND oi.name LIKE $2"#,
        )?;
        let res = stmt
            .query_map(
                rusqlite::params![board_id, format!("%{search}%")],
                |value| {
                    let size: i64 = value.get("extract_size")?;
                    Ok(bb_imager_ui::img_selection::ImageItem {
                        id: bb_imager_ui::img_selection::ImageId::OsImage(value.get("id")?),
                        label: value.get("name")?,
                        icon: value.get::<_, Option<Url>>("icon")?.map(Arc::new),
                        description: value.get("description")?,
                        size: Some(size as u64),
                        release_date: value.get("release_date")?,
                    })
                },
            )?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    /// Reconstruct the [`config::Device`] entry for a board, as it was inserted from the config.
    ///
    /// Statements here are not cached: this only runs when the user copies the
    /// flashing summary to the clipboard, so a cache slot would be wasted on it.
    pub(crate) fn os_board_json_by_id(&self, id: i64) -> rusqlite::Result<config::Device> {
        let db = self.db.lock().unwrap();

        let mut stmt = db.prepare("SELECT tag FROM board_tags WHERE board_id = $1")?;
        let tags = stmt
            .query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;

        let mut stmt = db.prepare(
            r#"
            SELECT name, description, icon, flasher, instructions, oshw, specification, documentation
            FROM boards WHERE id = $1"#)?;

        stmt.query_one([id], |value| {
            let spec: Vec<u8> = value.get("specification")?;

            Ok(config::Device {
                name: value.get("name")?,
                description: value.get("description")?,
                icon: value.get("icon")?,
                flasher: value.get("flasher")?,
                instructions: value.get("instructions")?,
                specification: serde_json::from_slice(&spec).unwrap(),
                documentation: value.get("documentation")?,
                oshw: value.get("oshw")?,
                tags,
            })
        })
    }

    /// Reconstruct the [`config::OsImage`] entry for an image, as it was inserted from the config.
    ///
    /// Image tags are not stored, and `devices` is recovered from the boards the image ended up
    /// linked to, so it can contain more tags than the original config entry listed.
    ///
    /// Statements here are not cached, for the same reason as
    /// [`Self::os_board_json_by_id`].
    pub(crate) fn os_image_json_by_id(&self, id: i64) -> rusqlite::Result<config::OsImage> {
        let db = self.db.lock().unwrap();

        let mut stmt = db.prepare(
            r#"
            SELECT DISTINCT bt.tag
            FROM os_image_boards oib
            JOIN board_tags bt ON bt.board_id = oib.board_id
            WHERE oib.image_id = $1"#,
        )?;
        let devices = stmt
            .query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;

        let mut stmt = db.prepare(
            r#"
            SELECT name, description, icon, url, image_download_size, image_download_sha256,
                extract_size, release_date, init_format, bmap, info_text, support
            FROM os_images WHERE id = $1"#,
        )?;

        stmt.query_one([id], |value| {
            let image_download_size: Option<i64> = value.get("image_download_size")?;
            let extract_size: i64 = value.get("extract_size")?;

            Ok(config::OsImage {
                name: value.get("name")?,
                description: value.get("description")?,
                icon: value.get("icon")?,
                url: value.get("url")?,
                image_download_size: image_download_size.map(|x| x as u64),
                image_download_sha256: value.get("image_download_sha256")?,
                extract_size: extract_size as u64,
                release_date: value.get("release_date")?,
                init_format: value.get("init_format")?,
                bmap: value.get("bmap")?,
                info_text: value.get("info_text")?,
                support: value.get("support")?,
                devices,
            })
        })
    }
}
