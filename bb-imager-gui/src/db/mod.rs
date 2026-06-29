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

#[derive(Debug, Clone)]
pub(crate) struct BoardListItem {
    pub(crate) id: i64,
    pub(crate) icon: Option<Url>,
    pub(crate) name: String,
}

impl BoardListItem {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            icon: value.get("icon")?,
            name: value.get("name")?,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Board {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) icon: Option<Url>,
    pub(crate) description: String,
    pub(crate) documentation: Option<Url>,
    pub(crate) specification: Vec<(String, String)>,
    pub(crate) oshw: Option<String>,
    pub(crate) flasher: config::Flasher,
    pub(crate) instructions: Option<String>,
}

impl Board {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let spec: Vec<u8> = value.get("specification")?;

        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            icon: value.get("icon")?,
            description: value.get("description")?,
            documentation: value.get("documentation")?,
            specification: serde_json::from_slice(&spec).unwrap(),
            oshw: value.get("oshw")?,
            flasher: value.get("flasher")?,
            instructions: value.get("instructions")?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OsImageListItem {
    pub(crate) id: i64,
    pub(crate) icon: Url,
    pub(crate) name: String,
}

impl OsImageListItem {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            icon: value.get("icon")?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OsSublistListItem {
    pub(crate) id: i64,
    pub(crate) icon: Url,
    pub(crate) name: String,
    pub(crate) flasher: bb_config::config::Flasher,
}

impl OsSublistListItem {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            icon: value.get("icon")?,
            flasher: value.get("flasher")?,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct OsImage {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) icon: Url,
    pub(crate) url: Url,
    pub(crate) image_download_size: Option<i64>,
    pub(crate) image_download_sha256: [u8; 32],
    pub(crate) extract_size: i64,
    pub(crate) release_date: chrono::NaiveDate,
    pub(crate) init_format: bb_config::config::InitFormat,
    pub(crate) bmap: Option<Url>,
    pub(crate) info_text: Option<String>,
    pub(crate) support: Option<Url>,
}

impl OsImage {
    fn from_row(value: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: value.get("id")?,
            name: value.get("name")?,
            icon: value.get("icon")?,
            description: value.get("description")?,
            url: value.get("url")?,
            image_download_size: value.get("image_download_size")?,
            image_download_sha256: value.get("image_download_sha256")?,
            extract_size: value.get("extract_size")?,
            release_date: value.get("release_date")?,
            init_format: value.get("init_format")?,
            bmap: value.get("bmap")?,
            info_text: value.get("info_text")?,
            support: value.get("support")?,
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
        let mut stmt = exec.prepare(
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
        exec.execute(
            "UPDATE remote_configs SET fetched = TRUE WHERE id = $1",
            [id],
        )?;
        Ok(())
    }

    pub(crate) fn remote_configs(&self) -> rusqlite::Result<Vec<(i64, Url)>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT id, url FROM remote_configs WHERE fetched = FALSE")?;
        let res = stmt
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

        tx.execute(
            "UPDATE os_sublists SET subitems_url = NULL WHERE id = $1",
            [id],
        )?;

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
        exec.execute(
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
            [sublist_id],
        )?;
        Ok(())
    }

    fn insert_sublist_boards(
        exec: &Connection,
        parent_id: i64,
        image_id: i64,
    ) -> rusqlite::Result<()> {
        exec.execute(
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
            [parent_id, image_id],
        )?;
        Ok(())
    }

    fn insert_sub_list(
        exec: &Connection,
        item: &config::OsSubList,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let mut stmt = exec.prepare(
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
        let mut stmt = exec.prepare(
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

        // Insert new tags
        let mut stmt = exec.prepare(
            r#"
            INSERT INTO board_tags(board_id, tag)
            VALUES ($1, $2)
            "#,
        )?;
        for tag in &board.tags {
            stmt.execute(rusqlite::params![id, tag])?;
        }

        Ok(())
    }

    fn insert_image(
        exec: &Connection,
        img: &config::OsImage,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let mut stmt = exec.prepare(
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

        let mut stmt = exec.prepare(
            r#"
            INSERT INTO os_image_boards(image_id, board_id)
            SELECT $1, b.board_id
            FROM board_tags b
            WHERE b.tag = $2
                "#,
        )?;
        for dev in &img.devices {
            stmt.execute(rusqlite::params![id, dev])?;
        }

        Ok(id)
    }

    fn insert_remote_image(
        exec: &Connection,
        img: &config::OsRemoteSubList,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let mut stmt = exec.prepare(
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

        let mut stmt = exec.prepare(
            r#"
            INSERT INTO os_sublist_boards(sublist_id, board_id)
            SELECT $1, b.board_id
            FROM board_tags b
            WHERE b.tag = $2
                "#,
        )?;
        for dev in &img.devices {
            stmt.execute(rusqlite::params![id, dev])?;
        }

        Ok(id)
    }

    /// Get all board icons.
    pub(crate) fn board_icons(&self) -> rusqlite::Result<Vec<url::Url>> {
        let db = self.db.lock().unwrap();
        let mut stmt =
            db.prepare_cached("SELECT DISTINCT icon FROM boards WHERE icon IS NOT NULL")?;
        let res = stmt
            .query_map([], |r| r.get(0))?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    /// Get board list data. (ID, Icon, Name)
    pub(crate) fn board_list(&self, search: &str) -> rusqlite::Result<Vec<BoardListItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            "SELECT id, icon, name FROM boards WHERE name LIKE $1 COLLATE NOCASE",
        )?;
        let res = stmt
            .query_map([format!("%{}%", search)], BoardListItem::from_row)?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    pub(crate) fn board_by_id(&self, id: i64) -> rusqlite::Result<Board> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
        SELECT id, name, icon, description, documentation, specification, oshw, 
            flasher, instructions
        FROM boards
        WHERE id = $1"#,
        )?;
        stmt.query_row([id], Board::from_row)
    }

    pub(crate) fn os_image_by_id(&self, id: i64) -> rusqlite::Result<OsImage> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT id, name, description, icon, url, image_download_size,
                image_download_sha256, extract_size, release_date, init_format,
                bmap, info_text, support
            FROM os_images WHERE id = $1"#,
        )?;
        stmt.query_row([id], OsImage::from_row)
    }

    pub(crate) fn os_image_items(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<crate::helpers::OsImageItem>> {
        let a = self.os_images_by_board_id(board_id, parent_id)?;
        let b = self.os_sublists(board_id, parent_id)?;

        Ok(a.into_iter()
            .map(Into::into)
            .chain(b.into_iter().map(Into::into))
            .collect())
    }

    fn os_images_by_board_id(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<OsImageListItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT oi.id, oi.name, oi.icon
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
            .query_map(
                rusqlite::params![board_id, parent_id],
                OsImageListItem::from_row,
            )?
            .map(|x| x.unwrap())
            .collect();

        Ok(res)
    }

    fn os_sublists(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> rusqlite::Result<Vec<OsSublistListItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare_cached(
            r#"
            SELECT s.id, s.name, s.icon, s.flasher
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
            .query_map(
                rusqlite::params![board_id, parent_id],
                OsSublistListItem::from_row,
            )?
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
        let mut stmt = db.prepare(
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

    pub(crate) fn os_image_icons_by_board_id(&self, board_id: i64) -> rusqlite::Result<Vec<Url>> {
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
            .query_map([board_id], |r| r.get(0))?
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
        let mut stmt = db.prepare(
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
        let mut stmt = db.prepare(
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
    ) -> rusqlite::Result<Vec<crate::helpers::OsImageItem>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            r#"
            SELECT oi.id, oi.name, oi.icon
            FROM os_images oi
            JOIN os_image_boards oib ON oi.id = oib.image_id
            WHERE oib.board_id = $1 AND oi.name LIKE $2"#,
        )?;
        let res = stmt
            .query_map(
                rusqlite::params![board_id, format!("%{search}%")],
                OsImageListItem::from_row,
            )?
            .map(|x| x.unwrap())
            .map(Into::into)
            .collect();

        Ok(res)
    }
}
