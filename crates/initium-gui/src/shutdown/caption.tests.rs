use super::*;

pub(crate) fn verify_themed_shutdown_frame(theme: Theme) {
    let caption =
        Caption::load(ShutdownAction::Poweroff, None, None, theme).expect("load fallback caption");
    let (width, height) = (800u16, 100u16);
    let stride = u32::from(width) * 4;
    let mut pixels = vec![0u32; usize::from(width) * usize::from(height)];
    // SAFETY: The byte slice spans the initialized Vec allocation and has exclusive access.
    let buffer = unsafe {
        std::slice::from_raw_parts_mut(pixels.as_mut_ptr().cast::<u8>(), pixels.len() * 4)
    };
    caption
        .render_into(buffer, (width, height), stride)
        .expect("render shutdown caption");

    assert_eq!(&buffer[..3], &[0x30, 0x20, 0x10]);
    assert!(
        buffer
            .chunks_exact(4)
            .any(|pixel| pixel[..3] == [0, 0, 255])
    );
}

#[test]
fn checks_caption_geometry_from_configuration() {
    let file = std::env::temp_dir().join(format!("initium-caption-{}.bgra", std::process::id()));
    let size = [12, 8];
    std::fs::write(&file, [0; 12 * 8 * 4]).expect("write synthetic BGRA image");
    let theme = Theme::default();
    let valid = Caption::load(ShutdownAction::Reboot, Some(&file), Some(size), theme);
    let invalid = Caption::load(ShutdownAction::Reboot, Some(&file), Some([12, 9]), theme);
    std::fs::remove_file(file).expect("remove synthetic image");
    assert_eq!(valid.expect("load matching caption").size, size);
    assert!(matches!(invalid, Err(error) if error.kind() == io::ErrorKind::InvalidInput));
}
