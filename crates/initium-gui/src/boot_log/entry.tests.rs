use super::*;

#[test]
fn parses_kernel_timestamp_priority_and_escapes() {
    let entry = Entry::parse(br"12,90,5012345,-;touch: \xd0\xa2\x09ready")
        .expect("valid kernel log record");

    assert_eq!(entry.timestamp.as_c_str(), c"5.012");
    assert_eq!(entry.message.as_c_str(), c"touch: Т ready");
    assert_eq!(entry.level.badge().0, c"!");
    assert!(Entry::parse(b"bad record").is_none());
}
