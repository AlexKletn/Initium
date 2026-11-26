use std::io;

use initium_config::Config;
use initium_gui::{DisplayConnection, DisplayPort, DrmKms, Fbdev, connect_display, initialize};

#[test]
fn rejects_unavailable_display_devices() {
    let fbdev = Fbdev::new("/dev/null");
    let drm = DrmKms::new("/dev/null", None);
    let ports: [&dyn DisplayPort<Connection = DisplayConnection>; 2] = [&fbdev, &drm];

    for port in ports {
        let error = port.connect().err().expect("LVGL is uninitialized");
        assert!(error.to_string().contains("not initialized"));
    }

    initialize().expect("LVGL should initialize");

    for port in ports.into_iter().cycle().take(4) {
        let error = port.connect().err().expect("/dev/null is not a display");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(error.to_string().contains("/dev/null"));
    }
    for backend in ["fbdev", "drm-kms"] {
        let document = format!("[ui.display]\nbackend = {backend:?}\npath = \"/dev/null\"\n");
        let config = Config::parse(&document).expect("valid display settings");
        let display = config.ui.display.expect("configured display");
        let error = connect_display(&display)
            .err()
            .expect("/dev/null is not a display");
        assert!(error.to_string().contains("/dev/null"));
    }
}
