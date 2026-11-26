use initium_config::Config;

use super::*;

#[test]
fn scales_cover_and_preserves_pixels_outside_it() {
    let config = Config::parse(
        r#"
        [ui.splash.bootloader_frame.cover]
        reference_width = 8
        reference_height = 4
        x = 2
        y = 2
        width = 4
        height = 2
        rgb = [10, 20, 30]
        "#,
    )
    .expect("valid frame configuration");
    let cover = config
        .ui
        .splash
        .bootloader_frame
        .and_then(|frame| frame.cover)
        .expect("configured cover");
    let mut frame = BootloaderFrame::new(4, 2, vec![0x00332211; 8]);
    frame.cover(&cover).expect("cover fits");
    assert_eq!(
        frame.pixels(),
        &[0x00332211; 4]
            .into_iter()
            .chain([0x00332211, 0xff0a141e, 0xff0a141e, 0x00332211])
            .collect::<Vec<_>>()
    );
}

#[test]
fn rejects_non_framebuffer_devices() {
    let config = BootloaderFrameConfig {
        capture_device: None,
        cover: None,
        reserved_memory: None,
    };
    let result = BootloaderFrame::capture(Path::new("/dev/null"), &config);
    assert!(matches!(result, Err(error) if error.raw_os_error() == Some(libc::ENOTTY)));
}
