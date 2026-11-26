use super::*;

#[test]
fn accepts_a_shared_palette_with_partial_overrides() {
    assert_eq!(Theme::parse("").unwrap(), Theme::default());
    assert_eq!(
        Theme::parse("accent_rgb = 0x2468ac").unwrap(),
        Theme {
            accent_rgb: 0x2468ac,
            ..Theme::default()
        }
    );
    assert_eq!(
        Theme::parse("background_rgb = 0x1000000")
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
