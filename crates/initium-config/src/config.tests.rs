use std::path::Path;

use super::Config;

#[test]
fn parses_splash_assets() {
    let config = Config::parse(
        r#"
        [ui.splash]
        brand_text = "Example Linux"
        intro_frames = "/assets/intro.frames"
        loop_frames = "/assets/loop.frames"
        "#,
    )
    .expect("valid splash configuration");

    let splash = config.ui.splash;
    assert_eq!(splash.brand_text.as_deref(), Some("Example Linux"));
    assert_eq!(
        splash.intro_frames.as_deref(),
        Some(Path::new("/assets/intro.frames"))
    );
    assert_eq!(
        splash.loop_frames.as_deref(),
        Some(Path::new("/assets/loop.frames"))
    );
}

#[test]
fn parses_bootloader_cover() {
    let config = Config::parse(
        r#"
        [ui.splash.bootloader_frame.cover]
        reference_width = 1920
        reference_height = 1200
        x = 760
        y = 930
        width = 400
        height = 96
        rgb = [0, 0, 0]
        "#,
    )
    .expect("valid bootloader cover configuration");

    let cover = config
        .ui
        .splash
        .bootloader_frame
        .and_then(|frame| frame.cover)
        .expect("configured cover");
    assert_eq!(
        (cover.reference_width.get(), cover.reference_height.get()),
        (1920, 1200)
    );
    assert_eq!(
        (cover.x, cover.y, cover.width.get(), cover.height.get()),
        (760, 930, 400, 96)
    );
    assert_eq!(cover.rgb, [0, 0, 0]);
}

#[test]
fn parses_userspace_choices_in_order() {
    assert!(Config::default().ui.userspace.is_empty());

    let config = Config::parse(
        r#"
        [[ui.userspace]]
        name = "Linux"
        icon = "/assets/linux.png"

        [[ui.userspace]]
        name = "Recovery"
        icon = "/assets/recovery.png"
        "#,
    )
    .expect("valid userspace choices");

    let choices: Vec<_> = config
        .ui
        .userspace
        .iter()
        .map(|choice| (choice.name.as_str(), choice.icon.as_path()))
        .collect();
    assert_eq!(
        choices,
        [
            ("Linux", Path::new("/assets/linux.png")),
            ("Recovery", Path::new("/assets/recovery.png")),
        ]
    );
}

#[test]
fn parses_power_command_arguments() {
    let config = Config::parse(
        r#"
        [[ui.power_menu]]
        name = "Reboot"
        icon = "/assets/reboot.png"
        command = { program = "/usr/bin/systemctl", args = ["reboot"] }
        "#,
    )
    .expect("valid power menu");

    let item = &config.ui.power_menu[0];
    assert_eq!(item.name, "Reboot");
    assert_eq!(item.icon, Path::new("/assets/reboot.png"));
    assert_eq!(item.command.program, Path::new("/usr/bin/systemctl"));
    assert_eq!(item.command.args, ["reboot"]);
}

#[test]
fn parses_display_backend_and_rejects_mismatched_settings() {
    use crate::DisplayConfig;

    assert!(
        Config::parse("")
            .expect("empty config")
            .ui
            .display
            .is_none()
    );
    let config = Config::parse(
        r#"
        [ui.display]
        backend = "drm-kms"
        path = "/dev/dri/card1"
        connector_id = 42
        "#,
    )
    .expect("DRM configuration");
    assert!(matches!(
        config.ui.display,
        Some(DisplayConfig::DrmKms { path, connector_id: Some(42) })
            if path == Path::new("/dev/dri/card1")
    ));

    assert!(
        Config::parse(
            r#"
            [ui.display]
            backend = "fbdev"
            path = "/dev/fb0"
            connector_id = 42
            "#,
        )
        .is_err()
    );
}

#[test]
fn combines_display_animation_hardware_and_menus() {
    let config = Config::parse(
        r#"
        [ui]
        touch_path = "/dev/input/event0"

        [ui.display]
        backend = "drm-kms"
        path = "/dev/dri/card0"

        [ui.splash]
        loop_frames = "/assets/loop.frames"

        [ui.splash.animation]
        width = 640
        height = 480
        intro_fps = 30
        loop_fps = 30
        [ui.splash.animation.outro]
        merge_ms = 360
        collapse_ms = 240
        core_radius_px = 80
        region_radius_px = 150
        satellite_rgb_weights = [0, 1, 0]

        [[ui.hardware]]
        name = "Audio"
        ready = { glob = "/sys/class/sound/card*" }

        [[ui.userspace]]
        name = "Linux"
        icon = "/assets/linux.png"

        [[ui.power_menu]]
        name = "Restart"
        icon = "/assets/reboot.png"
        command = { program = "/usr/bin/systemctl", args = ["reboot"] }
        "#,
    )
    .expect("parse combined UI configuration");

    assert!(matches!(
        config.ui.display,
        Some(crate::DisplayConfig::DrmKms { .. })
    ));
    assert!(
        config
            .ui
            .splash
            .animation
            .expect("animation")
            .outro
            .expect("outro")
            .fits(640, 480)
    );
    assert!(config.ui.hardware[0].ready.is_some());
    assert_eq!(config.ui.userspace[0].name, "Linux");
    assert_eq!(config.ui.power_menu[0].command.args, ["reboot"]);
}
