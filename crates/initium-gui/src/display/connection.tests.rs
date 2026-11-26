use std::os::unix::ffi::OsStrExt;

use super::*;
use crate::initialize;

const ICON: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240, 31, 0,
    5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn renders_choices_and_dispatches_power_menu_selection() {
    initialize().expect("initialize LVGL");
    crate::lvgl::tests::assert_already_initialized();
    let display = DisplayConnection::open(Path::new("/dev/null"), create_display, |_, _| {
        sys::lv_result_t_LV_RESULT_OK
    })
    .expect("create a headless display")
    .with_theme(Theme {
        background_rgb: 0x102030,
        accent_rgb: 0xff0000,
    });
    assert!(
        display
            .show_userspace_selector(&[])
            .expect("empty choices")
            .is_none()
    );

    let path = std::env::temp_dir().join(format!("initium-icon-{}.png", std::process::id()));
    std::fs::write(&path, ICON).expect("write fixture");
    let source = CString::new([b"S:", path.as_os_str().as_bytes()].concat()).expect("icon path");
    let choices = ["Linux", "Android"].map(|name| UserspaceChoice {
        name: name.into(),
        icon: path.clone(),
    });
    let selector = display.show_userspace_selector(&choices);
    let rendered = selector.as_ref().is_ok_and(|selector| selector.is_some());
    if rendered {
        with_session(|_| {
            // SAFETY: The display and selector are live and the LVGL session is exclusive.
            unsafe {
                let screen = sys::lv_display_get_screen_active(display.as_ptr());
                let panel = sys::lv_obj_get_child(screen, 0);
                assert!(!panel.is_null());
                let tile = sys::lv_obj_get_child(panel, 0);
                assert!(!tile.is_null());
                let icon = sys::lv_obj_get_child(tile, 0);
                let label = sys::lv_obj_get_child(tile, 1);
                assert!(!icon.is_null() && !label.is_null());
                let icon_source = sys::lv_image_get_src(icon);
                assert!(!icon_source.is_null());
                assert_eq!(CStr::from_ptr(icon_source.cast()), source.as_c_str());
                assert_eq!(CStr::from_ptr(sys::lv_label_get_text(label)), c"Linux");
                let android = sys::lv_obj_get_child(panel, 1);
                assert!(!android.is_null());
                let android_label = sys::lv_obj_get_child(android, 1);
                assert!(!android_label.is_null());
                sys::lv_obj_update_layout(screen);
                assert_eq!(sys::lv_obj_get_y(tile), sys::lv_obj_get_y(android));
                assert_eq!(sys::lv_obj_get_x(android) - sys::lv_obj_get_x(tile), 228);
                assert_eq!(sys::lv_obj_get_width(icon), 44);
                assert_eq!(sys::lv_obj_get_width(android_label), 144);
                assert_eq!(
                    CStr::from_ptr(sys::lv_label_get_text(android_label)),
                    c"Android"
                );
            }
        })
        .expect("inspect selector");
    }
    drop(selector);
    verify_power_menu(&display, &path);
    verify_hardware_panel(&display, &path);
    verify_boot_log(&display);
    verify_bootloader_frame(&display);
    verify_boot_menu_navigation(&display, &path);
    crate::shutdown::verify_themed_shutdown_frame(display.theme());
    std::fs::remove_file(path).expect("remove fixture");
    assert!(rendered, "load the configured icon and create the selector");
}

unsafe extern "C" fn create_display() -> *mut sys::lv_display_t {
    // SAFETY: DisplayConnection::open calls this in an exclusive LVGL session.
    unsafe { sys::lv_display_create(2560, 1600) }
}

fn verify_power_menu(display: &DisplayConnection, icon: &Path) {
    use initium_config::PowerCommand;

    let items = [("/bin/false", "Power off"), ("/bin/sleep", "Restart")].map(|(program, name)| {
        PowerMenuItem {
            name: name.into(),
            icon: icon.into(),
            command: PowerCommand {
                program: program.into(),
                args: if program == "/bin/sleep" {
                    vec!["30".into()]
                } else {
                    Vec::new()
                },
            },
        }
    });
    let menu = display
        .show_power_menu(&items)
        .expect("create power menu")
        .expect("visible menu");
    assert!(
        menu.spawn_pending_command()
            .expect("no click yet")
            .is_none()
    );
    with_session(|_| {
        // SAFETY: The menu and display are live and access to LVGL is exclusive.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let panel = sys::lv_obj_get_child(screen, 0);
            assert!(!panel.is_null());
            let tile = sys::lv_obj_get_child(panel, 1);
            assert!(!tile.is_null());
            sys::lv_obj_update_layout(screen);
            assert_eq!(sys::lv_obj_get_width(tile), 192);
            assert_eq!(sys::lv_obj_get_height(tile), 192);
            sys::lv_obj_send_event(
                tile,
                sys::lv_event_code_t_LV_EVENT_CLICKED,
                std::ptr::null_mut(),
            );
        }
    })
    .expect("send click event");
    let mut child = menu
        .spawn_pending_command()
        .expect("spawn test command")
        .expect("clicked an entry");
    let running = child.try_wait().expect("poll test command").is_none();
    if running {
        child.kill().expect("stop test command");
        child.wait().expect("reap test command");
    }
    assert!(running);
    assert!(
        menu.spawn_pending_command()
            .expect("click consumed")
            .is_none()
    );
}

fn verify_hardware_panel(display: &DisplayConnection, detected: &Path) {
    use initium_config::Config;

    let status_path = detected.with_extension("state");
    let config = Config::parse(&format!(
        r#"
        [[ui.hardware]]
        name = "Touch"
        detected = {{ glob = {detected:?} }}
        ready = {{ path = {status_path:?}, equals = "running" }}
        "#
    ))
    .expect("parse hardware description");

    let panel = display
        .show_hardware(&config.ui.hardware)
        .expect("open hardware panel")
        .expect("nonempty hardware list");
    assert_hardware_label(display, c"Detected");
    std::fs::write(&status_path, "running\n").expect("write state");
    panel.update().expect("update hardware states");
    std::fs::remove_file(status_path).expect("remove state fixture");
    assert_hardware_label(display, c"Ready");
}

fn assert_hardware_label(display: &DisplayConnection, expected: &CStr) {
    with_session(|_| {
        // SAFETY: The test keeps the display and hardware panel live.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let panel = sys::lv_obj_get_child(screen, 0);
            assert!(!panel.is_null());
            let row = sys::lv_obj_get_child(panel, 0);
            assert!(!row.is_null());
            let label = sys::lv_obj_get_child(row, 1);
            assert!(!label.is_null());
            assert_eq!(CStr::from_ptr(sys::lv_label_get_text(label)), expected);
        }
    })
    .expect("inspect hardware label");
}

fn verify_boot_log(display: &DisplayConnection) {
    use crate::boot_log::entry::Entry;

    let entries = [
        Entry::parse(b"6,1,1000000,-;boot started").expect("first record"),
        Entry::parse(b"6,2,2500000,-;storage ready").expect("second record"),
    ];
    let _panel = crate::BootLogPanel::render(display, entries).expect("render boot log");

    with_session(|_| {
        // SAFETY: The panel and its parent display remain live under exclusive LVGL access.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let panel = sys::lv_obj_get_child(screen, 0);
            assert!(!panel.is_null());
            let row = sys::lv_obj_get_child(panel, 1);
            assert!(!row.is_null());
            let timestamp = sys::lv_obj_get_child(row, 0);
            let message = sys::lv_obj_get_child(row, 2);
            assert!(!timestamp.is_null() && !message.is_null());
            assert_eq!(CStr::from_ptr(sys::lv_label_get_text(timestamp)), c"2.500");
            assert_eq!(
                CStr::from_ptr(sys::lv_label_get_text(message)),
                c"storage ready"
            );
        }
    })
    .expect("inspect boot log");
}

fn verify_bootloader_frame(display: &DisplayConnection) {
    let frame = BootloaderFrame::new(2, 1, vec![0xff102030, 0xff405060]);
    let mut image = display
        .show_bootloader_frame(&frame)
        .expect("mount frame as image");
    image
        .show_brand("Example Linux")
        .expect("render boot brand");

    with_session(|_| {
        // SAFETY: The image, owned descriptor, and backing frame are kept live by the test.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            assert_eq!(
                sys::lv_color_to_u32(sys::lv_obj_get_style_bg_color(screen, 0)) & 0xff_ffff,
                display.theme().background_rgb
            );
            let image = sys::lv_obj_get_child(screen, 0);
            assert!(!image.is_null());
            let descriptor = sys::lv_image_get_src(image).cast::<sys::lv_image_dsc_t>();
            assert!(!descriptor.is_null());
            let descriptor = &*descriptor;
            assert_eq!((descriptor.header.w(), descriptor.header.h()), (2, 1));
            assert_eq!(descriptor.header.stride(), 8);
            assert_eq!(descriptor.data.cast::<u32>().read(), 0xff102030);
            let label = sys::lv_obj_get_child(image, 0);
            assert!(!label.is_null());
            assert_eq!(
                sys::lv_color_to_u32(sys::lv_obj_get_style_text_color(label, 0)) & 0xff_ffff,
                display.theme().accent_rgb
            );
            assert_eq!(
                CStr::from_ptr(sys::lv_label_get_text(label)),
                c"Example Linux"
            );
        }
    })
    .expect("inspect frame source");
}

fn verify_boot_menu_navigation(display: &DisplayConnection, icon: &Path) {
    use std::time::{Duration, Instant};

    let config = initium_config::UiConfig {
        userspace: ["Linux", "Android"]
            .map(|name| UserspaceChoice {
                name: name.into(),
                icon: icon.into(),
            })
            .into(),
        ..Default::default()
    };
    let mut menu = crate::BootMenu::new(display, &config).expect("create the menu");
    let now = Instant::now();
    assert_logo_target(display, 800);
    send_menu_click(display, None);
    assert_eq!(menu.update(now).expect("first tap"), 0);
    send_menu_click(display, None);
    assert_eq!(menu.update(now).expect("start menu transition"), 0);

    let opened = now + Duration::from_millis(250);
    assert_eq!(menu.update(opened).expect("open menu"), -560);
    assert_menu_tile_size(display);
    verify_userspace_state(display, 0, Some(1));
    menu.update(opened).expect("select Android");
    verify_userspace_state(display, 1, None);
    assert_logo_target(display, 240);
    send_menu_click(display, Some(0));
    menu.update(opened).expect("open the journal");
    assert_menu_layers(display, 4);
    send_menu_click(display, Some(0));
    menu.update(opened).expect("close the journal");
    assert_menu_layers(display, 3);

    send_menu_click(display, None);
    send_menu_click(display, None);
    assert_eq!(menu.update(opened).expect("start closing"), -560);
    assert_eq!(
        menu.update(opened + Duration::from_millis(250))
            .expect("close menu"),
        0
    );
    let closed = opened + Duration::from_millis(250);
    send_menu_click(display, None);
    send_menu_click(display, None);
    menu.update(closed).expect("reopen");
    menu.update(closed + Duration::from_millis(250))
        .expect("show selector again");
    verify_userspace_state(display, 1, None);
    assert!(menu.lock_userspace());
    assert!(!menu.lock_userspace());
    assert_menu_layers(display, 2);

    let locked = closed + Duration::from_millis(250);
    send_menu_click(display, None);
    send_menu_click(display, None);
    menu.update(locked).expect("close after locking");
    let settled = locked + Duration::from_millis(250);
    menu.update(settled).expect("close transition");
    send_menu_click(display, None);
    send_menu_click(display, None);
    menu.update(settled).expect("reopen after locking");
    menu.update(settled + Duration::from_millis(250))
        .expect("show actions after locking");
    assert_menu_layers(display, 2);
}

fn assert_logo_target(display: &DisplayConnection, y: i32) {
    with_session(|_| {
        // SAFETY: The display and the gesture target are live in this session.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let target = sys::lv_obj_get_child(screen, 1);
            assert!(!target.is_null());
            sys::lv_obj_update_layout(screen);
            assert!(sys::lv_obj_hit_test(
                target,
                &sys::lv_point_t { x: 1280, y }
            ));
            assert!(!sys::lv_obj_hit_test(
                target,
                &sys::lv_point_t {
                    x: 1380,
                    y: y + 100
                },
            ));
        }
    })
    .expect("verify the circular gesture target");
}

fn assert_menu_layers(display: &DisplayConnection, layers: u32) {
    with_session(|_| {
        // SAFETY: The display and menu are live in this session.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            assert_eq!(sys::lv_obj_get_child_count(screen), layers);
        }
    })
    .expect("inspect menu layers");
}

fn send_menu_click(display: &DisplayConnection, button: Option<i32>) {
    with_session(|_| {
        // SAFETY: The test owns a live display and its menu in this exclusive session.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let target = button.map_or_else(
                || sys::lv_obj_get_child(screen, 1),
                |index| {
                    let controls = sys::lv_obj_get_child(screen, 0);
                    assert!(!controls.is_null());
                    sys::lv_obj_get_child(controls, index)
                },
            );
            assert!(!target.is_null());
            sys::lv_obj_send_event(
                target,
                sys::lv_event_code_t_LV_EVENT_CLICKED,
                std::ptr::null_mut(),
            );
        }
    })
    .expect("dispatch menu click");
}

fn assert_menu_tile_size(display: &DisplayConnection) {
    with_session(|_| {
        // SAFETY: The display and visible menu remain live in this session.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let menu = sys::lv_obj_get_child(screen, 0);
            assert!(!menu.is_null());
            let tile = sys::lv_obj_get_child(menu, 0);
            assert!(!tile.is_null());
            sys::lv_obj_update_layout(screen);
            assert_eq!(sys::lv_obj_get_width(tile), 192);
            assert_eq!(sys::lv_obj_get_height(tile), 192);
        }
    })
    .expect("verify the tile geometry");
}

fn verify_userspace_state(display: &DisplayConnection, selected: usize, click: Option<i32>) {
    with_session(|_| {
        // SAFETY: The selector and its two tiles are kept live by the test menu.
        unsafe {
            let screen = sys::lv_display_get_screen_active(display.as_ptr());
            let selector = sys::lv_obj_get_child(screen, 2);
            assert!(!selector.is_null());
            for index in 0..2 {
                let tile = sys::lv_obj_get_child(selector, index as i32);
                assert!(!tile.is_null());
                assert_eq!(
                    sys::lv_obj_has_state(tile, sys::lv_state_t_LV_STATE_CHECKED),
                    index == selected
                );
            }
            if let Some(index) = click {
                let tile = sys::lv_obj_get_child(selector, index);
                sys::lv_obj_send_event(
                    tile,
                    sys::lv_event_code_t_LV_EVENT_CLICKED,
                    std::ptr::null_mut(),
                );
            }
        }
    })
    .expect("check the userspace selection");
}
