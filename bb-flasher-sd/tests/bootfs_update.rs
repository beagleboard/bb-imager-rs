#![cfg(feature = "mock_sd")]

use bb_flasher_sd::Destination;
use bb_flasher_sd::bootfs_update::flash;
use bb_flasher_sd::mock_sd::{MockArchive, MockContent};
use std::io::{Read, Write};
use tempfile::NamedTempFile;

const READER_CONTENTS: &str = "reader";
const APPEND_CONTENTS: &str = "append";

#[test]
fn test_flash_workflow_with_helper_inspection() {
    // 1. Initialize the public mock storage block device
    let mut mock_sd = bb_flasher_sd::mock_sd::MockSd::new();

    let temp_file_data = "Hello World";
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(temp_file_data.as_bytes()).unwrap();

    // 2. Setup archive and closure
    let archive = MockArchive::from_entries(vec![
        ("config_dir".into(), MockContent::Dir),
        (
            "config_dir/reader.txt".into(),
            MockContent::Reader(READER_CONTENTS.as_bytes().into()),
        ),
        (
            "config_dir/file.txt".into(),
            MockContent::File(temp_file.path().into()),
        ),
        // Same path as the reader above on purpose: appends onto the entry
        // created earlier in this same archive.
        (
            "config_dir/reader.txt".into(),
            MockContent::DataAppend(APPEND_CONTENTS.as_bytes().into()),
        ),
    ]);
    let img_closure = move || Ok(archive);

    // 3. Execute the public API over the MockSD's path
    let destination = Destination::File(mock_sd.path().into());
    let flash_result = flash(img_closure, destination, None);

    assert!(
        flash_result.is_ok(),
        "Flashing failed: {:?}",
        flash_result.err()
    );

    // 4. Use the new clean API to inspect side-effects
    let fs = mock_sd.open_boot();
    let root_dir = fs.root_dir();

    // 5. Assert the changes are present
    assert!(
        root_dir.open_dir("config_dir").is_ok(),
        "config directory missing"
    );

    let mut actual_contents = String::new();
    root_dir
        .open_file("config_dir/reader.txt")
        .unwrap()
        .read_to_string(&mut actual_contents)
        .unwrap();

    assert_eq!(actual_contents, [READER_CONTENTS, APPEND_CONTENTS].join(""));

    actual_contents.clear();
    root_dir
        .open_file("config_dir/file.txt")
        .unwrap()
        .read_to_string(&mut actual_contents)
        .unwrap();

    assert_eq!(actual_contents, temp_file_data);
}
