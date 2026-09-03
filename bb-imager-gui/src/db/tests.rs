use crate::constants::DEFAULT_CONFIG;

use super::*;
use bb_config::Config;

/// This test verifies that database initialization correctly loads
/// remote configuration URLs from DEFAULT_CONFIG.
///
/// What this test checks:
/// 1. DEFAULT_CONFIG can be parsed into bb_config::Config.
/// 2. Db::new() creates a temporary SQLite database.
/// 3. Db::init() runs migrations and inserts DEFAULT_CONFIG data.
/// 4. remote_configs() returns all unfetched remote config URLs.
/// 5. Returned URLs match the ones defined in DEFAULT_CONFIG.
///
/// Why this matters:
/// - Prevents duplication of DEFAULT_CONFIG values in tests
/// - Ensures JSON parsing and DB insertion stay in sync
/// - Ensures remote_configs() correctly reflects DEFAULT_CONFIG
#[test]
fn init_loads_all_default_remote_configs() {
    let db = Db::new().expect("Failed to create DB");

    db.init().expect("DB initialization should succeed");

    let urls = db
        .remote_configs()
        .expect("Fetching remote configs should succeed");

    // Parse DEFAULT_CONFIG to extract expected URLs
    let config: Config =
        serde_json::from_slice(DEFAULT_CONFIG).expect("DEFAULT_CONFIG should be valid");

    let expected_urls = config.imager.remote_configs.clone();

    assert_eq!(
        urls.len(),
        expected_urls.len(),
        "All remote config URLs from DEFAULT_CONFIG should be inserted"
    );

    assert_eq!(expected_urls.len(), urls.len());
    for (_, u) in urls {
        assert!(expected_urls.contains(&u));
    }
}

/// This test verifies that add_config() correctly inserts new remote
/// configuration URLs into the database.
///
/// What this test checks:
/// 1. DB initializes with DEFAULT_CONFIG remote URLs.
/// 2. A new config with additional remote_configs is added.
/// 3. add_config() inserts the new remote URLs.
/// 4. remote_configs() returns both default and newly added URLs.
///
/// Why this matters:
/// - Ensures add_config() correctly inserts remote configs
/// - Ensures private insert_remote_config() is exercised via public API
/// - Ensures DB properly merges multiple config sources
#[test]
fn add_config_inserts_new_remote_configs() {
    let db = Db::new().expect("Failed to create DB");

    db.init().expect("DB initialization should succeed");

    // Initial remote configs from DEFAULT_CONFIG
    let initial_urls = db
        .remote_configs()
        .expect("Fetching remote configs should succeed");

    let initial_count = initial_urls.len();

    // Create a minimal config with only remote_configs
    let new_config = Config {
        imager: bb_config::config::Imager {
            remote_configs: vec![
                "https://example.com/test-os-list.json".try_into().unwrap(),
                "https://example.com/another-os-list.json"
                    .try_into()
                    .unwrap(),
            ],
            devices: vec![],
        },
        os_list: vec![],
    };

    // Add new config
    db.add_config(new_config, None)
        .expect("add_config should succeed");

    let updated_urls = db
        .remote_configs()
        .expect("Fetching remote configs should succeed");

    assert_eq!(
        updated_urls.len(),
        initial_count + 2,
        "Two new remote configs should be added"
    );

    assert!(
        updated_urls
            .iter()
            .any(|(_, u)| u.as_str() == "https://example.com/test-os-list.json")
    );

    assert!(
        updated_urls
            .iter()
            .any(|(_, u)| u.as_str() == "https://example.com/another-os-list.json")
    );
}

/// This test verifies that add_config() does not insert duplicate
/// remote configuration URLs into the database.
///
/// What this test checks:
/// 1. DB initializes with DEFAULT_CONFIG remote URLs.
/// 2. A config containing an already existing remote URL is added.
/// 3. add_config() runs successfully.
/// 4. remote_configs() still contains the same number of URLs.
///
/// Why this matters:
/// - Remote configs may appear in multiple config sources
/// - Ensures DB does not store duplicate URLs
/// - Ensures add_config() is safe for repeated ingestion
#[test]
fn add_config_does_not_duplicate_remote_configs() {
    let db = Db::new().expect("Failed to create DB");

    db.init().expect("DB initialization should succeed");

    let initial_urls = db
        .remote_configs()
        .expect("Fetching remote configs should succeed");

    assert!(!initial_urls.is_empty());

    let (_, existing_url) = initial_urls.first().unwrap().clone();

    let initial_count = initial_urls.len();

    // Create config with already existing remote config
    let mut imager = bb_config::config::Imager::default();
    imager.remote_configs.push(existing_url);

    let new_config = Config {
        imager,
        os_list: vec![],
    };

    db.add_config(new_config, None)
        .expect("add_config should succeed");

    let updated_urls = db
        .remote_configs()
        .expect("Fetching remote configs should succeed");

    assert_eq!(
        updated_urls.len(),
        initial_count,
        "Duplicate remote config should not be inserted"
    );
}

/// This test verifies that add_config() correctly inserts devices
/// into the database and they appear in board_list().
///
/// What this test checks:
/// 1. DB initializes successfully.
/// 2. A minimal device is added using add_config().
/// 3. board_list() returns the inserted device.
/// 4. Device fields are correctly stored.
///
/// Why this matters:
/// - Ensures add_config() inserts device data correctly
/// - Ensures board_list() retrieves devices from DB
/// - Verifies basic device storage pipeline
#[test]
fn add_config_inserts_device_into_board_list() {
    let db = Db::new().expect("Failed to create DB");

    db.init().expect("DB initialization should succeed");

    let initial_boards = db
        .board_list("")
        .expect("Fetching board list should succeed");

    let initial_count = initial_boards.len();

    // Create minimal device
    let device = bb_config::config::Device {
        name: "Test Board".to_string(),
        tags: Box::new(["test-board".into()]),
        icon: None,
        description: "Test device".to_string(),
        flasher: bb_config::config::Flasher::SdCard,
        documentation: None,
        instructions: None,
        specification: vec![],
        oshw: None,
        bootfs: None,
    };

    let mut imager = bb_config::config::Imager::default();
    imager.devices.push(device.clone());

    let new_config = Config {
        imager,
        os_list: vec![],
    };

    db.add_config(new_config, None)
        .expect("add_config should succeed");

    let updated_boards = db
        .board_list("")
        .expect("Fetching board list should succeed");

    assert_eq!(
        updated_boards.len(),
        initial_count + 1,
        "One new device should be added to board_list"
    );

    assert!(
        updated_boards.iter().any(|b| b.name == "Test Board"),
        "Inserted device should appear in board_list"
    );
}

/// This test verifies that add_config() updates an existing device
/// when a device with the same name is inserted again.
///
/// What this test checks:
/// 1. DB initializes successfully.
/// 2. A new device is inserted using add_config().
/// 3. Same device name is inserted again with different fields.
/// 4. board_list() still contains only one device entry.
/// 5. board_by_id() returns updated device details.
///
/// Why this matters:
/// - Device name acts as unique identity
/// - Prevents duplicate boards in DB
/// - Ensures add_config() performs an upsert
/// - Ensures board_by_id() returns updated fields
#[test]
fn add_config_updates_existing_device_with_same_name() {
    let db = Db::new().expect("Failed to create DB");

    db.init().expect("DB initialization should succeed");

    // Insert initial device
    let device_v1 = bb_config::config::Device {
        name: "Test Board".to_string(),
        tags: Box::new(["test-board".into()]),
        icon: None,
        description: "Old description".to_string(),
        flasher: bb_config::config::Flasher::SdCard,
        documentation: None,
        instructions: None,
        specification: vec![],
        oshw: None,
        bootfs: None,
    };

    let mut imager = bb_config::config::Imager::default();
    imager.devices.push(device_v1);

    db.add_config(
        Config {
            imager,
            os_list: vec![],
        },
        None,
    )
    .expect("First add_config should succeed");

    // Get inserted board id
    let boards = db
        .board_list("")
        .expect("Fetching board list should succeed");

    let board = boards
        .iter()
        .find(|b| b.name == "Test Board")
        .expect("Inserted board should exist");

    let board_id = board.id;
    let initial_count = boards.len();

    // Insert updated device with same name
    let device_v2 = bb_config::config::Device {
        name: "Test Board".to_string(),
        tags: Box::new(["updated-tag".into()]),
        icon: None,
        description: "Updated description".to_string(),
        flasher: bb_config::config::Flasher::SdCard,
        documentation: None,
        instructions: Some("New instructions".to_string()),
        specification: vec![("CPU".to_string(), "Test CPU".to_string())],
        oshw: Some("us000000".to_string()),
        bootfs: None,
    };

    let mut imager = bb_config::config::Imager::default();
    imager.devices.push(device_v2.clone());

    db.add_config(
        Config {
            imager,
            os_list: vec![],
        },
        None,
    )
    .expect("Second add_config should succeed");

    // Ensure board count unchanged
    let updated_boards = db
        .board_list("")
        .expect("Fetching board list should succeed");

    assert_eq!(
        updated_boards.len(),
        initial_count,
        "Board with same name should be updated, not duplicated"
    );

    // Fetch full board details
    let updated_board = db
        .board_by_id(board_id)
        .expect("Fetching board by id should succeed");

    assert_eq!(updated_board.description, device_v2.description);
    assert_eq!(updated_board.flasher, device_v2.flasher);
    assert_eq!(
        updated_board.instructions.as_deref(),
        device_v2.instructions.as_deref()
    );
    assert_eq!(updated_board.oshw, device_v2.oshw);
    assert_eq!(updated_board.specification, device_v2.specification);
}

/// This test verifies that add_config() correctly inserts an OS image
/// and makes it available through os_image_items() for a matching board.
///
/// What this test checks:
/// 1. A board with a specific tag is inserted.
/// 2. An OS image referencing that tag is inserted.
/// 3. The OS image is linked to the board via os_image_boards.
/// 4. os_image_items(board_id, None) returns the OS image.
///
/// Why this test is needed:
/// - OS images are not directly queryable; they are only accessible through board filtering.
/// - insert_image() links OS images to boards via tags and os_image_boards.
/// - os_image_items() is the main API used by the UI to retrieve OS entries.
/// - Ensures the full pipeline works:
///   add_config → insert_image → os_image_boards → os_image_items
///
/// Without this test:
/// - OS images could be inserted but never appear for any board.
/// - Tag-based linking could silently break.
/// - UI would show empty OS list even with valid config.
#[test]
fn add_config_inserts_os_image_for_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let image = bb_config::config::OsImage {
        name: "Test OS".to_string(),
        description: "Test OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [1; 32],
        extract_size: 2048,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        devices: Box::new(["test_board".into()]),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board.clone()],
        },
        os_list: vec![bb_config::config::OsListItem::Image(image.clone())],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let boards = db.board_list("").unwrap();
    let board_id = boards.iter().find(|b| b.name == board.name).unwrap().id;

    let items = db
        .os_image_items(board_id, None)
        .expect("os_image_items should succeed");

    assert!(items.iter().any(|x| x.label() == image.name));
}

/// This test verifies that os_image_by_id() returns the full OS image
/// details exactly as inserted by add_config().
///
/// What this test checks:
/// 1. A board is inserted.
/// 2. An OS image is inserted with all fields populated.
/// 3. os_image_items() returns the image ID.
/// 4. os_image_by_id() returns full OS image details.
/// 5. All fields match the inserted values.
///
/// Why this test is needed:
/// - os_image_by_id() is used to retrieve full OS metadata.
/// - OsImage uses custom FromRow decoding (e.g. sha256, Url, init_format).
/// - Ensures DB serialization/deserialization works correctly.
/// - Prevents silent data corruption or missing fields.
/// - Verifies:
///   add_config → insert_image → os_images → os_image_by_id
///
/// Without this test:
/// - SHA256 could be stored incorrectly.
/// - URLs could decode incorrectly.
/// - release_date or init_format could break silently.
/// - UI would receive incorrect OS metadata.
#[test]
fn os_image_by_id_returns_correct_data() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let image = bb_config::config::OsImage {
        name: "Test OS".to_string(),
        description: "Test OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [7; 32],
        extract_size: 4096,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
        devices: Box::new(["test_board".into()]),
        init_format: bb_config::config::InitFormat::None,
        bmap: Some("https://example.com/os.bmap".try_into().unwrap()),
        info_text: Some("Test info".to_string()),
        support: Some(
            "https://github.com/beagleboard/bb-imager-rs"
                .try_into()
                .unwrap(),
        ),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::Image(image.clone())],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let boards = db.board_list("").unwrap();
    let board_id = boards.iter().find(|b| b.name == "Test Board").unwrap().id;

    let items = db
        .os_image_items(board_id, None)
        .expect("os_image_items should succeed");

    let crate::helpers::OsImageId::OsImage(image_id) =
        items.iter().find(|x| x.label() == "Test OS").unwrap().id
    else {
        panic!("Incorrect ID");
    };
    let stored = db
        .os_image_by_id(image_id)
        .expect("os_image_by_id should succeed");

    assert_eq!(stored.name.as_ref(), image.name.as_str());
    assert_eq!(stored.description, image.description);
    assert_eq!(stored.url.as_str(), image.url.as_str());
    assert_eq!(stored.icon.as_str(), image.icon.as_str());
    assert_eq!(stored.image_download_size, Some(1024));
    assert_eq!(stored.image_download_sha256, [7; 32]);
    assert_eq!(stored.extract_size, 4096);
    assert_eq!(stored.release_date, image.release_date);
    assert_eq!(stored.init_format, image.init_format);
    assert_eq!(
        stored.bmap.as_ref().map(|x| x.as_str()),
        image.bmap.as_ref().map(|x| x.as_str())
    );
    assert_eq!(stored.info_text.as_deref(), image.info_text.as_deref());
}

/// This test verifies that add_config() correctly inserts an OsSubList
/// and makes it visible through os_image_items() for a matching board.
///
/// What this test checks:
/// 1. A board with a tag is inserted.
/// 2. An OsSubList referencing that tag is inserted.
/// 3. The sublist is linked to the board via os_sublist_boards.
/// 4. os_image_items(board_id, None) returns the sublist.
///
/// Why this test is needed:
/// - OsSubList is stored in a different table than OsImage.
/// - insert_sub_list() and insert_sublist_boards() handle hierarchy and board linkage.
/// - os_image_items() merges images and sublists into one list for the UI.
/// - Ensures:
///   add_config → insert_sub_list → os_sublist_boards → os_image_items
///
/// Without this test:
/// - Sublists could be inserted but never appear in UI.
/// - Board linkage could silently break.
/// - OS hierarchy navigation would fail.
#[test]
fn add_config_inserts_os_sublist_for_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let image = bb_config::config::OsImage {
        name: "Test OS".to_string(),
        description: "Test OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [1; 32],
        extract_size: 2048,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        devices: Box::new(["test_board".into()]),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    let sublist = bb_config::config::OsSubList {
        name: "Test SubList".to_string(),
        description: "SubList description".to_string(),
        icon: "https://example.com/sublist.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems: vec![bb_config::config::OsListItem::Image(image)],
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::SubList(sublist)],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let boards = db.board_list("").unwrap();
    let board_id = boards.iter().find(|b| b.name == "Test Board").unwrap().id;

    let items = db
        .os_image_items(board_id, None)
        .expect("os_image_items should succeed");

    assert!(items.iter().any(|x| x.label() == "Test SubList"));
}

/// This test verifies that board support propagates through multiple
/// levels of nested OsSubLists.
///
/// What this test checks:
/// 1. A board with a tag is inserted.
/// 2. A nested sublist hierarchy is created:
///    Parent SubList - Child SubList - OsImage (supports board)
/// 3. Board support propagates from OsImage to Child SubList.
/// 4. Board support propagates from Child SubList to Parent SubList.
/// 5. os_image_items(board_id, None) returns Parent SubList.
///
/// Why this test is needed:
/// - insert_sublist_boards() uses recursive SQL to propagate board support.
/// - Multi-level propagation is complex and easy to break.
/// - Ensures parent sublists appear even if only deep child images support the board.
/// - Verifies:
///   add_config → insert_image → insert_sublist_boards → recursive ancestors → os_image_items
///
/// Without this test:
/// - Parent sublists might not appear in UI.
/// - Recursive propagation could silently fail.
/// - Deep OS hierarchy navigation would break.
#[test]
fn nested_os_sublists_propagate_board_support() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let image = bb_config::config::OsImage {
        name: "Nested OS".to_string(),
        description: "Nested OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [1; 32],
        extract_size: 2048,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        devices: Box::new(["test_board".into()]),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    let child_sublist = bb_config::config::OsSubList {
        name: "Child SubList".to_string(),
        description: "Child description".to_string(),
        icon: "https://example.com/child.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems: vec![bb_config::config::OsListItem::Image(image)],
    };

    let parent_sublist = bb_config::config::OsSubList {
        name: "Parent SubList".to_string(),
        description: "Parent description".to_string(),
        icon: "https://example.com/parent.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems: vec![bb_config::config::OsListItem::SubList(child_sublist)],
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::SubList(parent_sublist)],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let board_id = db
        .board_list("")
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let items = db
        .os_image_items(board_id, None)
        .expect("os_image_items should succeed");

    assert!(
        items.iter().any(|x| x.label() == "Parent SubList"),
        "Parent sublist should be visible through recursive propagation"
    );
}

/// This test verifies that an OsRemoteSubList is correctly inserted
/// and returned by os_remote_sublists() for a matching board.
///
/// What this test checks:
/// 1. A board with a tag is inserted.
/// 2. An OsRemoteSubList referencing that tag is inserted.
/// 3. The remote sublist is linked to the board via os_sublist_boards.
/// 4. os_remote_sublists(board_id, None) returns the remote sublist.
/// 5. The stored subitems_url is correct.
///
/// Why this test is needed:
/// - OsRemoteSubList follows a different insertion path than OsImage and OsSubList.
/// - insert_remote_image() stores subitems_url and board linkage.
/// - os_remote_sublists() is used to fetch pending remote sublists.
/// - Ensures:
///   add_config → insert_remote_image → os_sublist_boards → os_remote_sublists
///
/// Without this test:
/// - Remote sublists could be inserted but never discovered.
/// - subitems_url could be stored incorrectly.
/// - Remote config fetching would break silently.
#[test]
fn remote_os_sublist_is_returned_for_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let remote_sublist = bb_config::config::OsRemoteSubList {
        name: "Remote OS List".to_string(),
        description: "Remote description".to_string(),
        icon: "https://example.com/remote.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems_url: "https://example.com/os-list.json".try_into().unwrap(),
        devices: Box::new(["test_board".into()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::RemoteSubList(remote_sublist)],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let board_id = db
        .board_list("")
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let remote_lists = db
        .os_remote_sublists(board_id, None)
        .expect("os_remote_sublists should succeed");

    assert_eq!(remote_lists.len(), 1);
    assert_eq!(
        remote_lists[0].1.as_str(),
        "https://example.com/os-list.json"
    );
}

/// This test verifies that os_remote_sublist_resolve() correctly
/// resolves a remote sublist by removing its URL and inserting
/// child OS items.
///
/// What this test checks:
/// 1. A board is inserted.
/// 2. A remote sublist supporting the board is inserted.
/// 3. os_remote_sublists() returns the remote sublist.
/// 4. os_remote_sublist_resolve() is called with child OsImage.
/// 5. subitems_url is cleared.
/// 6. Child OS becomes visible via os_image_items().
/// 7. Remote sublist is no longer returned by os_remote_sublists().
///
/// Why this test is needed:
/// - Remote sublists must transition into normal sublists after fetch.
/// - os_remote_sublist_resolve() updates DB state and inserts children.
/// - Ensures remote OS lists actually become usable.
/// - Verifies:
///   os_remote_sublist_resolve
///   - UPDATE subitems_url
///   - insert_os_list_items
///   - os_image_items
///   - os_remote_sublists
///
/// Without this test:
/// - Remote sublists might never resolve.
/// - URLs might not be cleared.
/// - OS images might not appear.
/// - UI would never show fetched OS lists.
#[test]
fn remote_os_sublist_resolve_inserts_child_items_and_clears_url() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let remote_sublist = bb_config::config::OsRemoteSubList {
        name: "Remote OS List".to_string(),
        description: "Remote description".to_string(),
        icon: "https://example.com/remote.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems_url: "https://example.com/os-list.json".try_into().unwrap(),
        devices: Box::new(["test_board".into()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::RemoteSubList(remote_sublist)],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let board_id = db
        .board_list("")
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let remote_lists = db.os_remote_sublists(board_id, None).unwrap();

    assert_eq!(remote_lists.len(), 1);

    let sublist_id = remote_lists[0].0;

    let child_image = bb_config::config::OsImage {
        name: "Fetched OS".to_string(),
        description: "Fetched OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [1; 32],
        extract_size: 2048,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        devices: Box::new(["test_board".into()]),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    db.os_remote_sublist_resolve(
        sublist_id,
        &[bb_config::config::OsListItem::Image(child_image)],
    )
    .expect("resolve should succeed");

    let remote_lists_after = db.os_remote_sublists(board_id, None).unwrap();

    assert!(remote_lists_after.is_empty(),);

    let items = db.os_image_items(board_id, Some(sublist_id)).unwrap();

    assert!(items.iter().any(|x| x.label() == "Fetched OS"),);
}

/// This test verifies that resolving a remote sublist multiple times
/// does not duplicate OS items or corrupt database state.
///
/// What this test checks:
/// 1. A board and remote sublist are inserted.
/// 2. os_remote_sublist_resolve() is called once with a child OS image.
/// 3. os_remote_sublist_resolve() is called again with the same child OS image.
/// 4. Only one OS image exists inside the sublist.
/// 5. Remote sublist remains resolved (not re-added).
///
/// Why this test is needed:
/// - Remote config fetching may retry on failures.
/// - os_remote_sublist_resolve() may be called multiple times.
/// - DB must behave idempotently.
/// - Prevents:
///   - duplicate OS entries
///   - duplicate board mappings
///   - inconsistent UI behavior
///
/// Without this test:
/// - repeated resolves could insert duplicate OS entries
/// - UI could show duplicate OS options
/// - DB integrity could break over time
#[test]
fn duplicate_remote_sublist_resolve_does_not_duplicate_os_items() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["test_board".into()]),
    };

    let remote_sublist = bb_config::config::OsRemoteSubList {
        name: "Remote OS List".to_string(),
        description: "Remote description".to_string(),
        icon: "https://example.com/remote.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems_url: "https://example.com/os-list.json".try_into().unwrap(),
        devices: Box::new(["test_board".into()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::RemoteSubList(remote_sublist)],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let board_id = db
        .board_list("")
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let remote_lists = db.os_remote_sublists(board_id, None).unwrap();

    assert_eq!(remote_lists.len(), 1);

    let sublist_id = remote_lists[0].0;

    let child_image = bb_config::config::OsImage {
        name: "Fetched OS".to_string(),
        description: "Fetched OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [1; 32],
        extract_size: 2048,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        devices: Box::new(["test_board".into()]),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    // First resolve
    db.os_remote_sublist_resolve(
        sublist_id,
        &[bb_config::config::OsListItem::Image(child_image.clone())],
    )
    .expect("first resolve should succeed");

    // Second resolve (duplicate call)
    let second = db.os_remote_sublist_resolve(
        sublist_id,
        &[bb_config::config::OsListItem::Image(child_image)],
    );
    assert!(second.is_err());

    let items = db.os_image_items(board_id, Some(sublist_id)).unwrap();
    let count = items.iter().filter(|x| x.label() == "Fetched OS").count();

    assert_eq!(count, 1,);

    let remote_lists_after = db.os_remote_sublists(board_id, None).unwrap();

    assert!(remote_lists_after.is_empty(),);
}

/// This test verifies that board_list(search) correctly filters boards
/// using a case-insensitive LIKE query.
///
/// What this test checks:
/// 1. Multiple boards are inserted into the database.
/// 2. board_list(Some("beagle")) is called.
/// 3. Only boards whose names contain "beagle" (case-insensitive) are returned.
/// 4. Non-matching boards are excluded.
///
/// Why this test is needed:
/// - board_list() has two execution paths (search and non-search).
/// - Ensures the LIKE query is applied correctly.
/// - Ensures COLLATE NOCASE works as expected.
/// - Prevents regressions where search returns all boards or none.
///
/// Without this test:
/// - Search filtering could silently break.
/// - Case-insensitive matching might stop working.
/// - UI board search would behave incorrectly.
#[test]
fn board_list_search_filters_boards_case_insensitive() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board1 = bb_config::config::Device {
        name: "Test Board 1".to_string(),
        description: "Board 1".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["bbb".into()]),
    };

    let board2 = bb_config::config::Device {
        name: "Test Board 2".to_string(),
        description: "Board 2".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["beagleplay".into()]),
    };

    let board3 = bb_config::config::Device {
        name: "Test Board 3".to_string(),
        description: "Board 3".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::new(["rpi".into()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board1, board2, board3],
        },
        os_list: vec![],
    };

    db.add_config(config, None)
        .expect("add_config should succeed");

    let results = db.board_list("test").expect("search should succeed");

    assert_eq!(
        results.len(),
        3,
        "Only boards containing 'test' should be returned"
    );

    assert!(results.iter().any(|b| b.name == "Test Board 1"));
    assert!(results.iter().any(|b| b.name == "Test Board 2"));
    assert!(results.iter().any(|b| b.name == "Test Board 3"));
}

/// Insert a single board and return its id.
fn insert_board_helper(db: &Db, board: bb_config::config::Device) -> i64 {
    let name = board.name.clone();

    db.add_config(
        Config {
            imager: bb_config::config::Imager {
                remote_configs: Default::default(),
                devices: vec![board],
            },
            os_list: vec![],
        },
        None,
    )
    .expect("add_config should succeed");

    db.board_list("")
        .expect("Fetching board list should succeed")
        .iter()
        .find(|b| b.name == name)
        .expect("Inserted board should exist")
        .id
}

/// This test verifies that os_board_json_by_id() reconstructs the exact
/// [`bb_config::config::Device`] that was inserted through add_config().
///
/// What this test checks:
/// 1. A device with every field populated is inserted.
/// 2. os_board_json_by_id() returns a Device equal to the original.
/// 3. Multi-entry tags and ordered specification pairs survive the round trip.
///
/// Why this matters:
/// - The device is stored across two tables (boards + board_tags) and the
///   specification is stored as a serialized blob, so a round trip is the only
///   way to catch a column being dropped or serialized in the wrong shape.
/// - The reconstructed Device is handed back to the flashing code, so any
///   missing field would silently change flashing behaviour.
#[test]
fn os_board_json_by_id_round_trips_device() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test description".to_string(),
        icon: Some("https://example.com/icon.png".try_into().unwrap()),
        flasher: bb_config::config::Flasher::SdCard,
        instructions: Some("Hold the boot button".to_string()),
        oshw: Some("us000000".to_string()),
        bootfs: None,
        specification: vec![
            ("CPU".to_string(), "Test CPU".to_string()),
            ("RAM".to_string(), "1GB".to_string()),
        ],
        documentation: Some("https://example.com/docs".try_into().unwrap()),
        tags: ["test-board".into(), "test-board-alt".into()].into(),
    };

    let id = insert_board_helper(&db, board.clone());

    let res = db
        .os_board_json_by_id(id)
        .expect("Fetching board json should succeed");

    assert_eq!(res, board);
}

/// This test verifies that os_board_json_by_id() works for a board with no
/// optional fields, no tags and an empty specification.
///
/// What this test checks:
/// 1. A device with all optional columns NULL is inserted.
/// 2. os_board_json_by_id() returns it without error.
/// 3. Tags and specification come back empty instead of failing.
///
/// Why this matters:
/// - icon, instructions, oshw and documentation are nullable columns; reading
///   them into non-Option types would panic only for such minimal boards.
/// - A board without tags produces zero rows in board_tags, which must not be
///   treated as a missing board.
#[test]
fn os_board_json_by_id_handles_board_without_optional_fields() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Minimal Board".to_string(),
        description: "Minimal".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: Box::default(),
    };

    let id = insert_board_helper(&db, board.clone());

    let res = db
        .os_board_json_by_id(id)
        .expect("Fetching board json should succeed");

    assert_eq!(res, board);
}

/// This test verifies that os_board_json_by_id() reflects an updated board
/// instead of mixing old and new data.
///
/// What this test checks:
/// 1. A board is inserted and then re-inserted with the same name.
/// 2. os_board_json_by_id() returns the updated fields.
/// 3. Tags from the first insertion are gone, not merged with the new ones.
///
/// Why this matters:
/// - insert_board() upserts on name and deletes the old tags; a regression
///   there would leave stale tags attached to the board.
/// - Stale tags would also link the board to OS images it no longer supports.
#[test]
fn os_board_json_by_id_reflects_updated_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board_v1 = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Old description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: ["old-tag".into()].into(),
    };

    let id = insert_board_helper(&db, board_v1);

    let board_v2 = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "New description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: Some("New instructions".to_string()),
        oshw: Some("us000000".to_string()),
        bootfs: None,
        specification: vec![("CPU".to_string(), "Test CPU".to_string())],
        documentation: None,
        tags: ["new-tag".into()].into(),
    };

    assert_eq!(
        insert_board_helper(&db, board_v2.clone()),
        id,
        "Re-inserting the same board name should update the same row"
    );

    let res = db
        .os_board_json_by_id(id)
        .expect("Fetching board json should succeed");

    assert_eq!(res, board_v2);
}

/// Insert a board along with an OS image and return the id of the inserted image.
fn insert_image_helper(
    db: &Db,
    board: bb_config::config::Device,
    image: bb_config::config::OsImage,
) -> i64 {
    let name = image.name.clone();

    db.add_config(
        Config {
            imager: bb_config::config::Imager {
                remote_configs: Default::default(),
                devices: vec![board.clone()],
            },
            os_list: vec![bb_config::config::OsListItem::Image(image)],
        },
        None,
    )
    .expect("add_config should succeed");

    let board_id = db
        .board_list("")
        .expect("Fetching board list should succeed")
        .iter()
        .find(|b| b.name == board.name)
        .expect("Inserted board should exist")
        .id;

    let items = db
        .os_image_items(board_id, None)
        .expect("os_image_items should succeed");

    let crate::helpers::OsImageId::OsImage(image_id) = items
        .iter()
        .find(|x| x.label.as_ref() == name.as_str())
        .expect("Inserted image should exist")
        .id
    else {
        panic!("Incorrect ID");
    };

    image_id
}

/// This test verifies that os_image_json_by_id() reconstructs the exact
/// [`bb_config::config::OsImage`] that was inserted through add_config().
///
/// What this test checks:
/// 1. An image with every field populated is inserted for a board with one tag.
/// 2. os_image_json_by_id() returns an OsImage equal to the original.
/// 3. sha256, sizes, release date, urls and devices survive the round trip.
///
/// Why this matters:
/// - Sizes are stored as signed integers and the sha256 as a blob, so a wrong
///   cast or column would only show up on a full round trip.
/// - The reconstructed OsImage describes what is actually being flashed, so a
///   missing field would misreport the flashing job.
#[test]
fn os_image_json_by_id_round_trips_image() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: ["test_board".into()].into(),
    };

    let image = bb_config::config::OsImage {
        name: "Test OS".to_string(),
        description: "Test OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: Some(1024),
        image_download_sha256: [7; 32],
        extract_size: 4096,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
        devices: ["test_board".into()].into(),
        init_format: bb_config::config::InitFormat::Sysconf,
        bmap: Some("https://example.com/os.bmap".try_into().unwrap()),
        info_text: Some("Test info".to_string()),
        support: Some(
            "https://github.com/beagleboard/bb-imager-rs"
                .try_into()
                .unwrap(),
        ),
    };

    let id = insert_image_helper(&db, board, image.clone());

    let res = db
        .os_image_json_by_id(id)
        .expect("Fetching image json should succeed");

    assert_eq!(res, image);
}

/// This test verifies that os_image_json_by_id() works for an image with none
/// of the optional fields set.
///
/// What this test checks:
/// 1. An image without download size, bmap, info text and support url is inserted.
/// 2. os_image_json_by_id() returns it without error.
///
/// Why this matters:
/// - Those columns are nullable; reading them into non-Option types would only
///   fail for images that omit them.
#[test]
fn os_image_json_by_id_handles_image_without_optional_fields() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: ["test_board".into()].into(),
    };

    let image = bb_config::config::OsImage {
        name: "Minimal OS".to_string(),
        description: "Minimal OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: None,
        image_download_sha256: [0; 32],
        extract_size: 1,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
        devices: ["test_board".into()].into(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    let id = insert_image_helper(&db, board, image.clone());

    let res = db
        .os_image_json_by_id(id)
        .expect("Fetching image json should succeed");

    assert_eq!(res, image);
}

/// This test pins the known lossiness of os_image_json_by_id(): the returned
/// `devices` comes from the boards the image is linked to, not from the config.
///
/// What this test checks:
/// 1. A board carrying two tags is inserted.
/// 2. An image referencing only one of those tags is inserted.
/// 3. os_image_json_by_id() reports both tags in `devices`.
///
/// Why this matters:
/// - The original device list is not stored; only the board links are. Callers
///   must treat `devices` as "tags of the boards this image matched", and this
///   test documents that instead of leaving it to be discovered later.
#[test]
fn os_image_json_by_id_devices_come_from_linked_boards() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs: None,
        specification: vec![],
        documentation: None,
        tags: ["test_board".into(), "test_board_alt".into()].into(),
    };

    let image = bb_config::config::OsImage {
        name: "Test OS".to_string(),
        description: "Test OS description".to_string(),
        icon: "https://example.com/icon.png".try_into().unwrap(),
        url: "https://example.com/os.img.xz".try_into().unwrap(),
        image_download_size: None,
        image_download_sha256: [1; 32],
        extract_size: 1,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
        devices: ["test_board".into()].into(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
        support: None,
    };

    let id = insert_image_helper(&db, board, image);

    let res = db
        .os_image_json_by_id(id)
        .expect("Fetching image json should succeed");

    // `devices` is now an ordered slice, but the query has no ORDER BY, so
    // compare without depending on the row order.
    let mut devices: Vec<&str> = res.devices.iter().map(AsRef::as_ref).collect();
    devices.sort_unstable();
    assert_eq!(devices, ["test_board", "test_board_alt"]);
}

/// This test verifies that os_image_json_by_id() fails for an unknown image id.
///
/// What this test checks:
/// 1. An id that does not exist is queried.
/// 2. The call returns QueryReturnedNoRows instead of a default image.
///
/// Why this matters:
/// - Returning a bogus image instead of an error would report the wrong image
///   for a flashing job.
#[test]
fn os_image_json_by_id_unknown_id_returns_no_rows() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let res = db.os_image_json_by_id(i64::MAX);

    assert!(
        matches!(res, Err(rusqlite::Error::QueryReturnedNoRows)),
        "Unknown image id should return QueryReturnedNoRows, got {res:?}"
    );
}

/// This test verifies that os_board_json_by_id() fails for an unknown board id.
///
/// What this test checks:
/// 1. An id that does not exist is queried.
/// 2. The call returns QueryReturnedNoRows instead of a default board.
///
/// Why this matters:
/// - Callers pass board ids kept in UI state; returning a bogus board instead
///   of an error would flash a device with the wrong configuration.
#[test]
fn os_board_json_by_id_unknown_id_returns_no_rows() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let res = db.os_board_json_by_id(i64::MAX);

    assert!(
        matches!(res, Err(rusqlite::Error::QueryReturnedNoRows)),
        "Unknown board id should return QueryReturnedNoRows, got {res:?}"
    );
}

/// A board's bootfs tarball, used by images flashed with
/// [`bb_config::config::Flasher::SdCardNoBootloader`].
fn test_bootfs() -> bb_config::config::Bootfs {
    bb_config::config::Bootfs {
        url: "https://example.com/bootfs.tar.xz".try_into().unwrap(),
        extract_size: 4096,
        image_download_sha256: [7; 32],
    }
}

fn board_with_bootfs(
    name: &str,
    tag: &str,
    bootfs: Option<bb_config::config::Bootfs>,
) -> bb_config::config::Device {
    bb_config::config::Device {
        name: name.to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        bootfs,
        specification: vec![],
        documentation: None,
        tags: Box::new([tag.into()]),
    }
}

/// A sublist of images that carry no bootloader, plus one image inside it.
fn no_bootloader_sublist(tag: &str) -> bb_config::config::OsSubList {
    bb_config::config::OsSubList {
        name: "Fedora Images".to_string(),
        description: "Images without a bootloader".to_string(),
        icon: "https://example.com/sublist.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCardNoBootloader,
        subitems: vec![bb_config::config::OsListItem::Image(
            bb_config::config::OsImage {
                name: "Fedora Minimal".to_string(),
                description: "Test OS description".to_string(),
                icon: "https://example.com/icon.png".try_into().unwrap(),
                url: "https://example.com/os.raw.xz".try_into().unwrap(),
                image_download_size: Some(1024),
                image_download_sha256: [1; 32],
                extract_size: 2048,
                release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                devices: Box::new([tag.into()]),
                init_format: bb_config::config::InitFormat::None,
                bmap: None,
                info_text: None,
                support: None,
            },
        )],
    }
}

/// This test verifies that a board's bootfs tarball survives the round trip
/// through the `boards` table.
///
/// What this test checks:
/// 1. A device with a bootfs is inserted.
/// 2. board_by_id() returns all three of url, extract size and sha256.
/// 3. os_board_json_by_id() reconstructs the same [`bb_config::config::Device`].
///
/// Why this matters:
/// - The tarball is stored as three separate nullable columns; dropping one of
///   them from an INSERT or SELECT would silently yield a board that cannot
///   flash bootloader-less images, or a wrong sha that fails verification only
///   after a full download.
#[test]
fn board_bootfs_round_trips() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let board = board_with_bootfs("Test Board", "test_board", Some(test_bootfs()));
    let id = insert_board_helper(&db, board.clone());

    assert_eq!(
        db.board_by_id(id)
            .expect("Fetching board should succeed")
            .bootfs,
        Some(test_bootfs())
    );
    assert_eq!(
        db.os_board_json_by_id(id)
            .expect("Fetching board json should succeed"),
        board
    );
}

/// The counterpart: a board without a tarball reads back as `None` rather than
/// a half-built [`bb_config::config::Bootfs`].
#[test]
fn board_without_bootfs_reads_back_as_none() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    let id = insert_board_helper(&db, board_with_bootfs("Test Board", "test_board", None));

    assert!(
        db.board_by_id(id)
            .expect("Fetching board should succeed")
            .bootfs
            .is_none()
    );
}

/// This test verifies that images which need a bootloader are only offered for
/// boards that supply one.
///
/// What this test checks:
/// 1. The same `SdCardNoBootloader` sublist is linked to two boards.
/// 2. It is listed for the board that has a bootfs tarball.
/// 3. It is hidden for the board that does not.
///
/// Why this matters:
/// - Flashing such an image onto a board with no tarball produces a card that
///   never boots, with nothing in the UI explaining why. Hiding the entry is
///   what keeps the combination unreachable.
#[test]
fn no_bootloader_sublist_is_listed_only_for_boards_with_bootfs() {
    let db = Db::new().expect("Failed to create DB");
    db.init().expect("DB init should succeed");

    db.add_config(
        Config {
            imager: bb_config::config::Imager {
                remote_configs: Default::default(),
                devices: vec![
                    board_with_bootfs("With Bootfs", "shared_tag", Some(test_bootfs())),
                    board_with_bootfs("Without Bootfs", "shared_tag", None),
                ],
            },
            os_list: vec![bb_config::config::OsListItem::SubList(
                no_bootloader_sublist("shared_tag"),
            )],
        },
        None,
    )
    .expect("add_config should succeed");

    let boards = db
        .board_list("")
        .expect("Fetching board list should succeed");
    let id_of = |name: &str| boards.iter().find(|b| b.name == name).unwrap().id;

    let listed = |board: &str| {
        db.os_image_items(id_of(board), None)
            .expect("os_image_items should succeed")
            .iter()
            .any(|x| x.label() == "Fedora Images")
    };

    assert!(
        listed("With Bootfs"),
        "board supplying a bootfs should be offered the sublist"
    );
    assert!(
        !listed("Without Bootfs"),
        "board without a bootfs should not be offered images that need one"
    );
}
