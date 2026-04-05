//! This module handles interaction with sqlite db used for config.

use std::sync::Arc;

use bb_config::config;
use sqlx::{FromRow, Row, sqlite::SqliteRow};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct Db {
    _f: Arc<tempfile::NamedTempFile>,
    db: sqlx::SqlitePool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[repr(transparent)]
pub(crate) struct Url(url::Url);

impl sqlx::Type<sqlx::Sqlite> for Url {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl From<Url> for url::Url {
    fn from(value: Url) -> Self {
        value.0
    }
}

impl From<url::Url> for Url {
    fn from(value: url::Url) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for Url {
    type Target = url::Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Url {
    fn decode(
        value: <sqlx::Sqlite as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(Self(url::Url::parse(&s)?))
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct BoardListItem {
    pub(crate) id: i64,
    pub(crate) icon: Option<Url>,
    pub(crate) name: String,
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

impl FromRow<'_, SqliteRow> for Board {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let spec: Vec<u8> = row.try_get("specification")?;

        Ok(Self {
            id: row.try_get("id")?,
            icon: row.try_get("icon")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            documentation: row.try_get("documentation")?,
            specification: serde_json::from_slice(&spec).unwrap(),
            oshw: row.try_get("oshw")?,
            flasher: row.try_get("flasher")?,
            instructions: row.try_get("instructions")?,
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct OsImageListItem {
    pub(crate) id: i64,
    pub(crate) icon: Url,
    pub(crate) name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct OsSublistListItem {
    pub(crate) id: i64,
    pub(crate) icon: Url,
    pub(crate) name: String,
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
    pub(crate) release_date: sqlx::types::chrono::NaiveDate,
    pub(crate) init_format: bb_config::config::InitFormat,
    pub(crate) bmap: Option<Url>,
    pub(crate) info_text: Option<String>,
}

impl FromRow<'_, SqliteRow> for OsImage {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let sha: Vec<u8> = row.try_get("image_download_sha256")?;

        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            icon: row.try_get("icon")?,
            url: row.try_get("url")?,
            image_download_size: row.try_get("image_download_size")?,
            extract_size: row.try_get("extract_size")?,
            image_download_sha256: sha.try_into().unwrap(),
            release_date: row.try_get("release_date")?,
            init_format: row.try_get("init_format")?,
            bmap: row.try_get("bmap")?,
            info_text: row.try_get("info_text")?,
        })
    }
}

impl Db {
    pub(crate) fn new() -> sqlx::Result<Self> {
        let f = tempfile::NamedTempFile::new()?;
        tracing::info!("DB Path: {}", f.path().display());
        let db_opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory)
            .filename(f.path());

        let db = sqlx::SqlitePool::connect_lazy_with(db_opts);

        Ok(Self {
            _f: Arc::new(f),
            db,
        })
    }

    pub(crate) async fn init(&self) -> sqlx::Result<()> {
        // Run migrations
        sqlx::migrate!().run(&self.db).await?;

        // Populate initial data
        let cfg =
            serde_json::from_slice::<bb_config::config::Config>(crate::constants::DEFAULT_CONFIG)
                .expect("Failed to parse config");

        self.add_config(cfg, None).await
    }

    pub(crate) async fn add_config(
        &self,
        cfg: config::Config,
        remote_config_id: Option<i64>,
    ) -> sqlx::Result<()> {
        let mut tx = self.db.begin().await?;

        if let Some(x) = remote_config_id {
            Self::remote_config_fetched(&mut tx, x).await?;
        }

        Self::insert_remote_config(&mut tx, cfg.imager.remote_configs.iter()).await?;

        for dev in cfg
            .imager
            .devices
            .iter()
            .filter(|x| crate::helpers::flasher_supported(x.flasher))
        {
            Self::insert_board(&mut tx, dev).await?;
        }

        Self::insert_os_list_items(&mut tx, &cfg.os_list, None, remote_config_id).await?;

        tx.commit().await
    }

    async fn insert_remote_config(
        exec: &mut sqlx::SqliteConnection,
        remote_configs: impl Iterator<Item = &url::Url>,
    ) -> sqlx::Result<()> {
        for u in remote_configs {
            sqlx::query(
                r#"
                INSERT INTO remote_configs(url) VALUES ($1) 
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(u.as_str())
            .execute(&mut *exec)
            .await?;
        }

        Ok(())
    }

    async fn remote_config_fetched(exec: &mut sqlx::SqliteConnection, id: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE remote_configs SET fetched = TRUE WHERE id = $1")
            .bind(id)
            .execute(exec)
            .await?;

        Ok(())
    }

    pub(crate) async fn remote_configs(&self) -> sqlx::Result<Vec<(i64, Url)>> {
        sqlx::query_as("SELECT id, url FROM remote_configs WHERE fetched = FALSE")
            .fetch_all(&self.db)
            .await
    }

    pub(crate) async fn os_remote_sublist_resolve(
        &self,
        id: i64,
        subitems: &[bb_config::config::OsListItem],
    ) -> sqlx::Result<()> {
        let mut tx = self.db.begin().await?;

        sqlx::query("UPDATE os_sublists SET subitems_url = NULL WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        Self::insert_os_list_items(&mut tx, subitems, Some(id), None).await?;

        tx.commit().await
    }

    async fn insert_os_list_items(
        exec: &mut sqlx::SqliteConnection,
        items: &[config::OsListItem],
        start_pid: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> sqlx::Result<()> {
        let mut imgs = Vec::from_iter(items.iter().map(|x| (start_pid, x)));

        while let Some((pid, img)) = imgs.pop() {
            match img {
                config::OsListItem::Image(os_image) => {
                    let id = Self::insert_image(exec, os_image, pid).await?;
                    if let Some(p) = pid {
                        Self::insert_sublist_boards(exec, p, id).await?
                    }
                }
                config::OsListItem::SubList(os_sub_list) => {
                    let id =
                        Self::insert_sub_list(exec, os_sub_list, pid, remote_config_id).await?;
                    imgs.extend(os_sub_list.subitems.iter().map(|x| (Some(id), x)));
                }
                config::OsListItem::RemoteSubList(os_remote_sub_list) => {
                    let id =
                        Self::insert_remote_image(exec, os_remote_sub_list, pid, remote_config_id)
                            .await?;
                    Self::insert_remote_sublist_boards(exec, id).await?;
                }
            }
        }

        Ok(())
    }

    async fn insert_remote_sublist_boards(
        exec: &mut sqlx::SqliteConnection,
        sublist_id: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
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
        )
        .bind(sublist_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    async fn insert_sublist_boards(
        exec: &mut sqlx::SqliteConnection,
        parent_id: i64,
        image_id: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
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
        )
        .bind(parent_id)
        .bind(image_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    async fn insert_sub_list(
        exec: &mut sqlx::SqliteConnection,
        item: &config::OsSubList,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> sqlx::Result<i64> {
        let id = sqlx::query(
            r#"
             INSERT INTO os_sublists(parent_id, name, description, icon, flasher, remote_config_id)
             VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(parent_id)
        .bind(&item.name)
        .bind(&item.description)
        .bind(item.icon.as_str())
        .bind(item.flasher)
        .bind(remote_config_id)
        .execute(&mut *exec)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    async fn insert_board(
        exec: &mut sqlx::SqliteConnection,
        board: &config::Device,
    ) -> sqlx::Result<()> {
        let spec = serde_json::to_vec(&board.specification).unwrap();

        // Insert or update board
        let id: i64 = sqlx::query_scalar(
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
        )
        .bind(&board.name)
        .bind(&board.description)
        .bind(board.icon.as_ref().map(|x| x.as_str()))
        .bind(board.flasher)
        .bind(&board.instructions)
        .bind(&board.oshw)
        .bind(&spec)
        .bind(board.documentation.as_ref().map(|x| x.as_str()))
        .fetch_one(&mut *exec)
        .await?;

        // Remove old tags
        sqlx::query(
            r#"
        DELETE FROM board_tags
        WHERE board_id = $1
        "#,
        )
        .bind(id)
        .execute(&mut *exec)
        .await?;

        // Insert new tags
        for tag in &board.tags {
            sqlx::query(
                r#"
            INSERT INTO board_tags(board_id, tag)
            VALUES ($1, $2)
            "#,
            )
            .bind(id)
            .bind(tag)
            .execute(&mut *exec)
            .await?;
        }

        Ok(())
    }

    async fn insert_image(
        exec: &mut sqlx::SqliteConnection,
        img: &config::OsImage,
        parent_id: Option<i64>,
    ) -> sqlx::Result<i64> {
        let id = sqlx::query(
            r#"
            INSERT INTO os_images(name, parent_id, description, icon, url, 
                image_download_size, image_download_sha256, extract_size, 
                release_date, init_format, bmap, info_text) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(&img.name)
        .bind(parent_id)
        .bind(&img.description)
        .bind(img.icon.as_str())
        .bind(img.url.as_str())
        .bind(img.image_download_size.map(|x| i64::try_from(x).unwrap()))
        .bind(img.image_download_sha256.as_slice())
        .bind(i64::try_from(img.extract_size).unwrap())
        .bind(img.release_date)
        .bind(img.init_format)
        .bind(img.bmap.as_ref().map(|x| x.as_str()))
        .bind(&img.info_text)
        .execute(&mut *exec)
        .await?
        .last_insert_rowid();

        for dev in &img.devices {
            sqlx::query(
                r#"
            INSERT INTO os_image_boards(image_id, board_id)
            SELECT $1, b.board_id
            FROM board_tags b
            WHERE b.tag = $2
                "#,
            )
            .bind(id)
            .bind(dev)
            .execute(&mut *exec)
            .await?;
        }

        Ok(id)
    }

    async fn insert_remote_image(
        exec: &mut sqlx::SqliteConnection,
        img: &config::OsRemoteSubList,
        parent_id: Option<i64>,
        remote_config_id: Option<i64>,
    ) -> sqlx::Result<i64> {
        let id = sqlx::query(
            r#"
            INSERT INTO os_sublists(parent_id, name, description, icon, 
                flasher, subitems_url, remote_config_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(parent_id)
        .bind(&img.name)
        .bind(&img.description)
        .bind(img.icon.as_str())
        .bind(img.flasher)
        .bind(img.subitems_url.as_str())
        .bind(remote_config_id)
        .execute(&mut *exec)
        .await?
        .last_insert_rowid();

        for dev in &img.devices {
            sqlx::query(
                r#"
            INSERT INTO os_sublist_boards(sublist_id, board_id)
            SELECT $1, b.board_id
            FROM board_tags b
            WHERE b.tag = $2
                "#,
            )
            .bind(id)
            .bind(dev)
            .execute(&mut *exec)
            .await?;
        }

        Ok(id)
    }

    /// Get all board icons.
    pub(crate) async fn board_icons(&self) -> sqlx::Result<Vec<url::Url>> {
        let res: Vec<String> =
            sqlx::query_scalar("SELECT DISTINCT icon FROM boards WHERE icon IS NOT NULL")
                .fetch_all(&self.db)
                .await?;

        Ok(res
            .into_iter()
            .map(|x| x.as_str().try_into().unwrap())
            .collect())
    }

    /// Get board list data. (ID, Icon, Name)
    pub(crate) async fn board_list(&self, search: &str) -> sqlx::Result<Vec<BoardListItem>> {
        sqlx::query_as("SELECT id, icon, name FROM boards WHERE name LIKE $1 COLLATE NOCASE")
            .bind(format!("%{}%", search))
            .fetch_all(&self.db)
            .await
    }

    pub(crate) async fn board_by_id(&self, id: i64) -> sqlx::Result<Board> {
        sqlx::query_as(
            r#"
        SELECT id, name, icon, description, documentation, specification, oshw, 
            flasher, instructions
        FROM boards
        WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(&self.db)
        .await
    }

    pub(crate) async fn os_image_by_id(&self, id: i64) -> sqlx::Result<OsImage> {
        sqlx::query_as(
            r#"
            SELECT id, name, description, icon, url, image_download_size,
                image_download_sha256, extract_size, release_date, init_format,
                bmap, info_text 
            FROM os_images WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(&self.db)
        .await
    }

    pub(crate) async fn os_image_items(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> sqlx::Result<Vec<crate::helpers::OsImageItem>> {
        let a = self.os_images_by_board_id(board_id, parent_id).await?;
        let b = self.os_sublists(board_id, parent_id).await?;

        Ok(a.into_iter()
            .map(Into::into)
            .chain(b.into_iter().map(Into::into))
            .collect())
    }

    async fn os_images_by_board_id(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> sqlx::Result<Vec<OsImageListItem>> {
        sqlx::query_as(
            r#"
            SELECT oi.id, oi.name, oi.icon
            FROM os_images oi
            JOIN os_image_boards oib ON oi.id = oib.image_id
            WHERE oib.board_id = $1 
                AND (
                        ($2 IS NULL AND oi.parent_id IS NULL) 
                        OR oi.parent_id = $2
                )"#,
        )
        .bind(board_id)
        .bind(parent_id)
        .fetch_all(&self.db)
        .await
    }

    async fn os_sublists(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> sqlx::Result<Vec<OsSublistListItem>> {
        sqlx::query_as(
            r#"
            SELECT s.id, s.name, s.icon
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1
              AND (
                    ($2 IS NULL AND s.parent_id IS NULL)
                 OR s.parent_id = $2
              )"#,
        )
        .bind(board_id)
        .bind(parent_id)
        .fetch_all(&self.db)
        .await
    }

    pub(crate) async fn os_remote_sublists(
        &self,
        board_id: i64,
        parent_id: Option<i64>,
    ) -> sqlx::Result<Vec<(i64, Url)>> {
        sqlx::query_as(
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
        )
        .bind(board_id)
        .bind(parent_id)
        .fetch_all(&self.db)
        .await
    }

    pub(crate) async fn os_sublist_parent(&self, sublist_id: i64) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar(r#"SELECT parent_id FROM os_sublists WHERE id = $1"#)
            .bind(sublist_id)
            .fetch_one(&self.db)
            .await
    }

    pub(crate) async fn os_image_icons_by_board_id(&self, board_id: i64) -> sqlx::Result<Vec<Url>> {
        sqlx::query_scalar(
            r#"
            SELECT oi.icon FROM os_images oi 
            JOIN os_image_boards oib ON oi.id = oib.image_id 
            WHERE oib.board_id = $1

            UNION

            SELECT s.icon
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1"#,
        )
        .bind(board_id)
        .fetch_all(&self.db)
        .await
    }

    pub(crate) async fn os_remote_sublists_by_remote_config(
        &self,
        board_id: i64,
        remote_config_id: i64,
    ) -> sqlx::Result<Vec<(i64, Url)>> {
        sqlx::query_as(
            r#"
            SELECT s.id, s.subitems_url
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1 
                AND s.subitems_url IS NOT NULL 
                AND s.remote_config_id = $2"#,
        )
        .bind(board_id)
        .bind(remote_config_id)
        .fetch_all(&self.db)
        .await
    }

    pub(crate) async fn os_remote_sublists_by_board(
        &self,
        board_id: i64,
    ) -> sqlx::Result<Vec<(i64, Url)>> {
        sqlx::query_as(
            r#"
            SELECT s.id, s.subitems_url
            FROM os_sublists s
            JOIN os_sublist_boards sb ON sb.sublist_id = s.id
            WHERE sb.board_id = $1 AND s.subitems_url IS NOT NULL"#,
        )
        .bind(board_id)
        .fetch_all(&self.db)
        .await
    }

    pub(crate) async fn os_images_by_name(
        &self,
        board_id: i64,
        search: &str,
    ) -> sqlx::Result<Vec<crate::helpers::OsImageItem>> {
        let res: Vec<OsImageListItem> = sqlx::query_as(
            r#"
            SELECT oi.id, oi.name, oi.icon
            FROM os_images oi
            JOIN os_image_boards oib ON oi.id = oib.image_id
            WHERE oib.board_id = $1 AND oi.name LIKE $2"#,
        )
        .bind(board_id)
        .bind(format!("%{search}%"))
        .fetch_all(&self.db)
        .await?;

        Ok(res.into_iter().map(Into::into).collect())
    }
}
