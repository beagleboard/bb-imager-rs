#![cfg(feature = "mock_sd")]

use bb_flasher_sd::bootfs_update::flash;
use bb_flasher_sd::{ContentType, Destination};
use std::io::{self, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

struct MockArchive {
    file_path: Box<Path>,
}

impl MockArchive {
    const READER_CONTENTS: &'static str = "reader";
    const APPEND_CONTENTS: &'static str = "append";

    fn new(file_path: Box<Path>) -> Self {
        Self { file_path }
    }
}

impl<'b> IntoIterator for &'b mut MockArchive {
    type Item = (Box<str>, ContentType<'b>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'b>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            vec![
                ("config_dir".into(), ContentType::Dir),
                (
                    "config_dir/reader.txt".into(),
                    ContentType::Reader(Box::new(io::Cursor::new(MockArchive::READER_CONTENTS))),
                ),
                (
                    "config_dir/file.txt".into(),
                    ContentType::File(self.file_path.clone()),
                ),
                (
                    "config_dir/reader.txt".into(),
                    ContentType::DataAppend(MockArchive::APPEND_CONTENTS.as_bytes().into()),
                ),
            ]
            .into_iter(),
        )
    }
}

#[test]
fn test_flash_workflow_with_helper_inspection() {
    // 1. Initialize the public mock storage block device
    let mut mock_sd = bb_flasher_sd::mock_sd::MockSd::new();

    let temp_file_data = "Hello World";
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(temp_file_data.as_bytes()).unwrap();

    // 2. Setup archive and closure
    let archive = MockArchive::new(temp_file.path().into());
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

    assert_eq!(
        actual_contents,
        [MockArchive::READER_CONTENTS, MockArchive::APPEND_CONTENTS].join("")
    );

    actual_contents.clear();
    root_dir
        .open_file("config_dir/file.txt")
        .unwrap()
        .read_to_string(&mut actual_contents)
        .unwrap();

    assert_eq!(actual_contents, temp_file_data);
}
