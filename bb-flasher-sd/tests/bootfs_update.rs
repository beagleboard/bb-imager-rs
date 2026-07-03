use bb_flasher_sd::bootfs_update::flash;
use bb_flasher_sd::{ContentType, Destination};
use bb_helper::mock_sd::MockSd;
use std::io::{self, Read};

struct IntegrationMockArchive(Vec<(Box<str>, Option<Vec<u8>>)>);

impl Default for IntegrationMockArchive {
    fn default() -> Self {
        Self(vec![
            ("config".into(), None),
            ("config/cmdline.txt".into(), Some(b"console=ttyS0".to_vec())),
        ])
    }
}

impl<'b> IntoIterator for &'b mut IntegrationMockArchive {
    type Item = (Box<str>, ContentType<'b>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'b>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.0
                .iter()
                .map(|(p, f)| match f {
                    Some(x) => (
                        p.clone(),
                        ContentType::Reader(Box::new(io::Cursor::new(x.clone()))),
                    ),
                    None => (p.clone(), ContentType::Dir),
                })
                .collect::<Vec<Self::Item>>()
                .into_iter(),
        )
    }
}

#[test]
fn test_flash_workflow_with_helper_inspection() {
    // 1. Initialize the public mock storage block device
    let mut mock_sd = MockSd::new();

    // 2. Setup archive and closure
    let img_closure = || Ok(IntegrationMockArchive::default());

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
        root_dir.open_dir("config").is_ok(),
        "config directory missing"
    );

    let mut actual_contents = String::new();
    root_dir
        .open_file("config/cmdline.txt")
        .expect("cmdline.txt missing")
        .read_to_string(&mut actual_contents)
        .expect("Failed to read file contents");

    assert_eq!(actual_contents, "console=ttyS0");
}
