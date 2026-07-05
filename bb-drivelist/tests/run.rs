#[test]
fn basic() {
    let temp = bb_drivelist::drive_list().unwrap();
    assert!(temp.len() > 0);
}
