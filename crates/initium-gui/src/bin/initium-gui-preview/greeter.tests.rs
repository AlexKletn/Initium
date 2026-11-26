use super::replaced;

#[test]
fn keeps_boot_frame_until_a_different_frame_is_active() {
    assert!(!replaced(Some(101), Some(101)));
    assert!(!replaced(Some(101), None));
    assert!(replaced(Some(101), Some(204)));
}
