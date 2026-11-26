use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShutdownConfig {
    pub seat: Option<String>,
    pub caption_size: Option<[u16; 2]>,
    pub drm_device: PathBuf,
    pub connector_id_file: Option<PathBuf>,
    pub fallback_size: Option<[u16; 2]>,
    pub fallback_refresh_hz: Option<u32>,
    pub console_framebuffer: Option<PathBuf>,
    pub console_blank: Option<PathBuf>,
    pub backlight_brightness: Option<PathBuf>,
    pub poweroff_caption: Option<PathBuf>,
    pub reboot_caption: Option<PathBuf>,
}
