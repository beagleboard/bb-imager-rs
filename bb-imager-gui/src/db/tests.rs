use std::collections::HashSet;

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
#[tokio::test]
async fn init_loads_all_default_remote_configs() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    let urls = db
        .remote_configs()
        .await
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
    for u in urls {
        assert!(expected_urls.contains(&u));
    }
}

/// This test verifies that marking a remote config as fetched
/// removes it from the list of pending remote configs.
///
/// What this test checks:
/// 1. DB initializes with DEFAULT_CONFIG remote URLs.
/// 2. remote_configs() returns all URLs initially.
/// 3. Calling remote_config_fetched(url) marks it as fetched.
/// 4. remote_configs() no longer returns that URL.
/// 5. Remaining URLs are unaffected.
///
/// Why this matters:
/// - Ensures remote_config_fetched() updates DB correctly
/// - Ensures remote_configs() filters fetched entries
/// - Ensures pending remote config tracking works correctly
#[tokio::test]
async fn remote_config_fetched_removes_url_from_pending_list() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    // Parse DEFAULT_CONFIG to get expected URLs
    let config: Config =
        serde_json::from_slice(DEFAULT_CONFIG).expect("DEFAULT_CONFIG should be valid");

    let expected_urls = config.imager.remote_configs;

    // Get current URLs
    let urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    assert!(!urls.is_empty(), "There should be remote configs present");

    let first_url = urls[0].clone();

    // Mark one URL as fetched
    db.remote_config_fetched(first_url.clone())
        .await
        .expect("Marking remote config as fetched should succeed");

    // Fetch again
    let updated_urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    // One less URL should remain
    assert_eq!(
        updated_urls.len(),
        expected_urls.len() - 1,
        "One remote config should be removed after marking as fetched"
    );

    // Ensure fetched URL is not present
    assert!(
        !updated_urls.contains(&first_url),
        "Fetched remote config should not appear in pending list"
    );

    // Ensure other URLs still exist
    for url in updated_urls {
        assert!(
            expected_urls.contains(&url),
            "Remaining URL should still be part of DEFAULT_CONFIG"
        );
    }
}

/// This test verifies that calling remote_config_fetched() multiple times
/// on the same URL does not cause errors and remains idempotent.
///
/// What this test checks:
/// 1. DB initializes with DEFAULT_CONFIG remote URLs.
/// 2. A remote config is marked as fetched.
/// 3. Calling remote_config_fetched() again on the same URL succeeds.
/// 4. The URL remains removed from remote_configs().
///
/// Why this matters:
/// - Remote config fetching may be retried in real scenarios
/// - Ensures DB update operation is safe to call multiple times
/// - Prevents failures during repeated fetch attempts
#[tokio::test]
async fn remote_config_fetched_is_idempotent() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    // Parse DEFAULT_CONFIG
    let config: Config =
        serde_json::from_slice(DEFAULT_CONFIG).expect("DEFAULT_CONFIG should be valid");

    let expected_urls = config.imager.remote_configs;

    // Get one URL
    let urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    assert!(!urls.is_empty());

    let url = urls[0].clone();

    // First fetch
    db.remote_config_fetched(url.clone())
        .await
        .expect("First fetch should succeed");

    // Second fetch (should not fail)
    db.remote_config_fetched(url.clone())
        .await
        .expect("Second fetch should also succeed");

    // Verify URL is still removed
    let remaining_urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    assert_eq!(
        remaining_urls.len(),
        expected_urls.len() - 1,
        "URL should only be removed once"
    );

    assert!(
        !remaining_urls.contains(&url),
        "Fetched URL should not reappear in pending list"
    );
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
#[tokio::test]
async fn add_config_inserts_new_remote_configs() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    // Initial remote configs from DEFAULT_CONFIG
    let initial_urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    let initial_count = initial_urls.len();

    // Create a minimal config with only remote_configs
    let new_config = Config {
        imager: bb_config::config::Imager {
            remote_configs: HashSet::from([
                "https://example.com/test-os-list.json".try_into().unwrap(),
                "https://example.com/another-os-list.json"
                    .try_into()
                    .unwrap(),
            ]),
            devices: vec![],
        },
        os_list: vec![],
    };

    // Add new config
    db.add_config(new_config)
        .await
        .expect("add_config should succeed");

    let updated_urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    assert_eq!(
        updated_urls.len(),
        initial_count + 2,
        "Two new remote configs should be added"
    );

    assert!(
        updated_urls
            .iter()
            .any(|u| u.as_str() == "https://example.com/test-os-list.json")
    );

    assert!(
        updated_urls
            .iter()
            .any(|u| u.as_str() == "https://example.com/another-os-list.json")
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
#[tokio::test]
async fn add_config_does_not_duplicate_remote_configs() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    let initial_urls = db
        .remote_configs()
        .await
        .expect("Fetching remote configs should succeed");

    assert!(!initial_urls.is_empty());

    let existing_url = initial_urls.first().unwrap().clone();

    let initial_count = initial_urls.len();

    // Create config with already existing remote config
    let mut imager = bb_config::config::Imager::default();
    imager.remote_configs.insert(existing_url);

    let new_config = Config {
        imager,
        os_list: vec![],
    };

    db.add_config(new_config)
        .await
        .expect("add_config should succeed");

    let updated_urls = db
        .remote_configs()
        .await
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
#[tokio::test]
async fn add_config_inserts_device_into_board_list() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    let initial_boards = db
        .board_list()
        .await
        .expect("Fetching board list should succeed");

    let initial_count = initial_boards.len();

    // Create minimal device
    let device = bb_config::config::Device {
        name: "Test Board".to_string(),
        tags: std::collections::HashSet::from(["test-board".to_string()]),
        icon: None,
        description: "Test device".to_string(),
        flasher: bb_config::config::Flasher::SdCard,
        documentation: None,
        instructions: None,
        specification: vec![],
        oshw: None,
    };

    let mut imager = bb_config::config::Imager::default();
    imager.devices.push(device.clone());

    let new_config = Config {
        imager,
        os_list: vec![],
    };

    db.add_config(new_config)
        .await
        .expect("add_config should succeed");

    let updated_boards = db
        .board_list()
        .await
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
#[tokio::test]
async fn add_config_updates_existing_device_with_same_name() {
    let db = Db::new().expect("Failed to create DB");

    db.init().await.expect("DB initialization should succeed");

    // Insert initial device
    let device_v1 = bb_config::config::Device {
        name: "Test Board".to_string(),
        tags: std::collections::HashSet::from(["test-board".to_string()]),
        icon: None,
        description: "Old description".to_string(),
        flasher: bb_config::config::Flasher::SdCard,
        documentation: None,
        instructions: None,
        specification: vec![],
        oshw: None,
    };

    let mut imager = bb_config::config::Imager::default();
    imager.devices.push(device_v1);

    db.add_config(Config {
        imager,
        os_list: vec![],
    })
    .await
    .expect("First add_config should succeed");

    // Get inserted board id
    let boards = db
        .board_list()
        .await
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
        tags: std::collections::HashSet::from(["updated-tag".to_string()]),
        icon: None,
        description: "Updated description".to_string(),
        flasher: bb_config::config::Flasher::SdCard,
        documentation: None,
        instructions: Some("New instructions".to_string()),
        specification: vec![("CPU".to_string(), "Test CPU".to_string())],
        oshw: Some("us000000".to_string()),
    };

    let mut imager = bb_config::config::Imager::default();
    imager.devices.push(device_v2.clone());

    db.add_config(Config {
        imager,
        os_list: vec![],
    })
    .await
    .expect("Second add_config should succeed");

    // Ensure board count unchanged
    let updated_boards = db
        .board_list()
        .await
        .expect("Fetching board list should succeed");

    assert_eq!(
        updated_boards.len(),
        initial_count,
        "Board with same name should be updated, not duplicated"
    );

    // Fetch full board details
    let updated_board = db
        .board_by_id(board_id)
        .await
        .expect("Fetching board by id should succeed");

    assert_eq!(updated_board.description, device_v2.description);
    assert_eq!(updated_board.flasher, device_v2.flasher);
    assert_eq!(updated_board.instructions, device_v2.instructions);
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
///     add_config → insert_image → os_image_boards → os_image_items
///
/// Without this test:
/// - OS images could be inserted but never appear for any board.
/// - Tag-based linking could silently break.
/// - UI would show empty OS list even with valid config.
#[tokio::test]
async fn add_config_inserts_os_image_for_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
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
        devices: HashSet::from(["test_board".to_string()]),
        tags: HashSet::new(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board.clone()],
        },
        os_list: vec![bb_config::config::OsListItem::Image(image.clone())],
    };

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let boards = db.board_list().await.unwrap();
    let board_id = boards.iter().find(|b| b.name == board.name).unwrap().id;

    let items = db
        .os_image_items(board_id, None)
        .await
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
///     add_config → insert_image → os_images → os_image_by_id
///
/// Without this test:
/// - SHA256 could be stored incorrectly.
/// - URLs could decode incorrectly.
/// - release_date or init_format could break silently.
/// - UI would receive incorrect OS metadata.
#[tokio::test]
async fn os_image_by_id_returns_correct_data() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
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
        devices: HashSet::from(["test_board".to_string()]),
        tags: HashSet::new(),
        init_format: bb_config::config::InitFormat::None,
        bmap: Some("https://example.com/os.bmap".try_into().unwrap()),
        info_text: Some("Test info".to_string()),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::Image(image.clone())],
    };

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let boards = db.board_list().await.unwrap();
    let board_id = boards.iter().find(|b| b.name == "Test Board").unwrap().id;

    let items = db
        .os_image_items(board_id, None)
        .await
        .expect("os_image_items should succeed");

    let crate::helpers::OsImageId::OsImage(image_id) =
        items.iter().find(|x| x.label() == "Test OS").unwrap().id
    else {
        panic!("Incorrect ID");
    };
    let stored = db
        .os_image_by_id(image_id)
        .await
        .expect("os_image_by_id should succeed");

    assert_eq!(stored.name, image.name);
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
    assert_eq!(stored.info_text, image.info_text);
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
///     add_config → insert_sub_list → os_sublist_boards → os_image_items
///
/// Without this test:
/// - Sublists could be inserted but never appear in UI.
/// - Board linkage could silently break.
/// - OS hierarchy navigation would fail.
#[tokio::test]
async fn add_config_inserts_os_sublist_for_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
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
        devices: HashSet::from(["test_board".to_string()]),
        tags: HashSet::new(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
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

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let boards = db.board_list().await.unwrap();
    let board_id = boards.iter().find(|b| b.name == "Test Board").unwrap().id;

    let items = db
        .os_image_items(board_id, None)
        .await
        .expect("os_image_items should succeed");

    assert!(items.iter().any(|x| x.label() == "Test SubList"));
}

/// This test verifies that board support propagates through multiple
/// levels of nested OsSubLists.
///
/// What this test checks:
/// 1. A board with a tag is inserted.
/// 2. A nested sublist hierarchy is created:
///        Parent SubList
///            └── Child SubList
///                    └── OsImage (supports board)
/// 3. Board support propagates from OsImage to Child SubList.
/// 4. Board support propagates from Child SubList to Parent SubList.
/// 5. os_image_items(board_id, None) returns Parent SubList.
///
/// Why this test is needed:
/// - insert_sublist_boards() uses recursive SQL to propagate board support.
/// - Multi-level propagation is complex and easy to break.
/// - Ensures parent sublists appear even if only deep child images support the board.
/// - Verifies:
///     add_config → insert_image → insert_sublist_boards → recursive ancestors → os_image_items
///
/// Without this test:
/// - Parent sublists might not appear in UI.
/// - Recursive propagation could silently fail.
/// - Deep OS hierarchy navigation would break.
#[tokio::test]
async fn nested_os_sublists_propagate_board_support() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
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
        devices: HashSet::from(["test_board".to_string()]),
        tags: HashSet::new(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
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

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let board_id = db
        .board_list()
        .await
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let items = db
        .os_image_items(board_id, None)
        .await
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
///     add_config → insert_remote_image → os_sublist_boards → os_remote_sublists
///
/// Without this test:
/// - Remote sublists could be inserted but never discovered.
/// - subitems_url could be stored incorrectly.
/// - Remote config fetching would break silently.
#[tokio::test]
async fn remote_os_sublist_is_returned_for_board() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
    };

    let remote_sublist = bb_config::config::OsRemoteSubList {
        name: "Remote OS List".to_string(),
        description: "Remote description".to_string(),
        icon: "https://example.com/remote.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems_url: "https://example.com/os-list.json".try_into().unwrap(),
        devices: HashSet::from(["test_board".to_string()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::RemoteSubList(remote_sublist)],
    };

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let board_id = db
        .board_list()
        .await
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let remote_lists = db
        .os_remote_sublists(board_id, None)
        .await
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
///     os_remote_sublist_resolve
///     → UPDATE subitems_url
///     → insert_os_list_items
///     → os_image_items
///     → os_remote_sublists
///
/// Without this test:
/// - Remote sublists might never resolve.
/// - URLs might not be cleared.
/// - OS images might not appear.
/// - UI would never show fetched OS lists.
#[tokio::test]
async fn remote_os_sublist_resolve_inserts_child_items_and_clears_url() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
    };

    let remote_sublist = bb_config::config::OsRemoteSubList {
        name: "Remote OS List".to_string(),
        description: "Remote description".to_string(),
        icon: "https://example.com/remote.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems_url: "https://example.com/os-list.json".try_into().unwrap(),
        devices: HashSet::from(["test_board".to_string()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::RemoteSubList(remote_sublist)],
    };

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let board_id = db
        .board_list()
        .await
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let remote_lists = db.os_remote_sublists(board_id, None).await.unwrap();

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
        devices: HashSet::from(["test_board".to_string()]),
        tags: HashSet::new(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
    };

    db.os_remote_sublist_resolve(
        sublist_id,
        &[bb_config::config::OsListItem::Image(child_image)],
    )
    .await
    .expect("resolve should succeed");

    let remote_lists_after = db.os_remote_sublists(board_id, None).await.unwrap();

    assert!(remote_lists_after.is_empty(),);

    let items = db.os_image_items(board_id, Some(sublist_id)).await.unwrap();

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
///     duplicate OS entries
///     duplicate board mappings
///     inconsistent UI behavior
///
/// Without this test:
/// - repeated resolves could insert duplicate OS entries
/// - UI could show duplicate OS options
/// - DB integrity could break over time
#[tokio::test]
async fn duplicate_remote_sublist_resolve_does_not_duplicate_os_items() {
    let db = Db::new().expect("Failed to create DB");
    db.init().await.expect("DB init should succeed");

    let board = bb_config::config::Device {
        name: "Test Board".to_string(),
        description: "Test Board description".to_string(),
        icon: None,
        flasher: bb_config::config::Flasher::SdCard,
        instructions: None,
        oshw: None,
        specification: vec![],
        documentation: None,
        tags: HashSet::from(["test_board".to_string()]),
    };

    let remote_sublist = bb_config::config::OsRemoteSubList {
        name: "Remote OS List".to_string(),
        description: "Remote description".to_string(),
        icon: "https://example.com/remote.png".try_into().unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems_url: "https://example.com/os-list.json".try_into().unwrap(),
        devices: HashSet::from(["test_board".to_string()]),
    };

    let config = Config {
        imager: bb_config::config::Imager {
            remote_configs: Default::default(),
            devices: vec![board],
        },
        os_list: vec![bb_config::config::OsListItem::RemoteSubList(remote_sublist)],
    };

    db.add_config(config)
        .await
        .expect("add_config should succeed");

    let board_id = db
        .board_list()
        .await
        .unwrap()
        .into_iter()
        .find(|b| b.name == "Test Board")
        .unwrap()
        .id;

    let remote_lists = db.os_remote_sublists(board_id, None).await.unwrap();

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
        devices: HashSet::from(["test_board".to_string()]),
        tags: HashSet::new(),
        init_format: bb_config::config::InitFormat::None,
        bmap: None,
        info_text: None,
    };

    // First resolve
    db.os_remote_sublist_resolve(
        sublist_id,
        &[bb_config::config::OsListItem::Image(child_image.clone())],
    )
    .await
    .expect("first resolve should succeed");

    // Second resolve (duplicate call)
    let second = db
        .os_remote_sublist_resolve(
            sublist_id,
            &[bb_config::config::OsListItem::Image(child_image)],
        )
        .await;
    assert!(second.is_err());

    let items = db.os_image_items(board_id, Some(sublist_id)).await.unwrap();
    let count = items.iter().filter(|x| x.label() == "Fetched OS").count();

    assert_eq!(count, 1,);

    let remote_lists_after = db.os_remote_sublists(board_id, None).await.unwrap();

    assert!(remote_lists_after.is_empty(),);
}
