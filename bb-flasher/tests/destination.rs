use std::path::PathBuf;

use bb_flasher::Destination;

#[test]
fn destination_local_file_behaviour() {
    let dst = Destination::LocalFile(PathBuf::from("/tmp/os.img").into());
    assert_eq!(dst.size(), None);
    assert_eq!(dst.to_string(), "Save To File");
    // No size, so the label is just the name.
    assert_eq!(dst.label(), "Save To File".into());
}

/// Picking the right disk depends on this: the identifier is what a click
/// carries, and the list it indexes is re-enumerated every second.
#[test]
fn destination_identifiers_are_stable_and_distinct() {
    let a = Destination::LocalFile(PathBuf::from("/tmp/a.img").into());
    let b = Destination::LocalFile(PathBuf::from("/tmp/b.img").into());

    assert_eq!(a.identifier(), a.identifier());
    assert_ne!(a.identifier(), b.identifier());
}
