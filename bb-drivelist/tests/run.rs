//! Smoke test for the public `drive_list()` entry point.
//!
//! The number of drives is host-dependent (a minimal CI container may expose
//! none), so this asserts `drive_list()` succeeds and checks per-descriptor
//! invariants rather than a non-empty count. The field-level classification
//! logic (is_usb / is_removable / ... derived from lsblk JSON) needs a parse
//! seam to exercise from an integration test and is deferred.

#[test]
fn drive_list_succeeds_and_descriptors_are_well_formed() {
    let devices = bb_drivelist::drive_list().expect("drive_list should succeed");

    for device in &devices {
        // The enumerator identifies the backend and is always populated.
        assert!(
            !device.enumerator.is_empty(),
            "every descriptor should have an enumerator: {device:?}"
        );
        // The raw device identifier is always populated.
        assert!(
            !device.raw.is_empty(),
            "every descriptor should have a raw device id: {device:?}"
        );
    }
}
