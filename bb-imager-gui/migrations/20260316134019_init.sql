-- Add migration script here
CREATE TABLE remote_configs
(
        id INTEGER PRIMARY KEY ASC,
	url TEXT NOT NULL UNIQUE,
	-- Flag to indicate if the config has been resolved
	fetched INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE boards
(
	id INTEGER PRIMARY KEY ASC,
	name TEXT NOT NULL UNIQUE,
	description TEXT NOT NULL,
	icon TEXT,
	flasher TEXT NOT NULL,
	instructions TEXT,
	oshw TEXT,
	specification BLOB,
	documentation TEXT
) STRICT;

CREATE TABLE board_tags
(
	board_id INTEGER NOT NULL,
	tag TEXT NOT NULL,

	PRIMARY KEY (board_id, tag),
	FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE os_sublists (
	id INTEGER PRIMARY KEY ASC,
	parent_id INTEGER,              -- NULL = root
	
	name TEXT NOT NULL,
	description TEXT NOT NULL,
	icon TEXT NOT NULL,
	flasher TEXT NOT NULL,
	
	-- NULL = remote sublist
	subitems_url TEXT DEFAULT NULL,
        -- NULL = Not from remote config. Can be from a remote subitem.
        remote_config_id INTEGER DEFAULT NULL,
	
	FOREIGN KEY (parent_id) REFERENCES os_sublists(id) ON DELETE CASCADE,
        FOREIGN KEY (remote_config_id) REFERENCES remote_configs(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE os_sublist_boards (
	sublist_id INTEGER NOT NULL,
	board_id INTEGER NOT NULL,
	
	PRIMARY KEY (sublist_id, board_id),
	FOREIGN KEY (sublist_id) REFERENCES os_sublists(id) ON DELETE CASCADE,
	FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE os_images(
	id INTEGER PRIMARY KEY ASC,
	parent_id INTEGER,              -- NULL = root
	name TEXT NOT NULL,
	description TEXT NOT NULL,
	icon TEXT NOT NULL,
	url TEXT NOT NULL UNIQUE,
	image_download_size INTEGER,
	image_download_sha256 BLOB NOT NULL,
	extract_size INTEGER NOT NULL,
	release_date TEXT NOT NULL,
	init_format TEXT NOT NULL,
	bmap TEXT,
	info_text TEXT,

	FOREIGN KEY (parent_id) REFERENCES os_sublists(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE os_image_boards (
	image_id INTEGER NOT NULL,
	board_id INTEGER NOT NULL,
	
	PRIMARY KEY (image_id, board_id),
	FOREIGN KEY (image_id) REFERENCES os_images(id) ON DELETE CASCADE,
	FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
) STRICT;
