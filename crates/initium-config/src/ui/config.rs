use std::path::PathBuf;

use serde::Deserialize;

use super::{DisplayConfig, HardwareItem, PowerMenuItem, SplashConfig, UserspaceChoice};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub display: Option<DisplayConfig>,
    pub touch_path: Option<PathBuf>,
    pub boot_state_dir: Option<PathBuf>,
    pub hardware: Vec<HardwareItem>,
    pub splash: SplashConfig,
    pub userspace: Vec<UserspaceChoice>,
    pub power_menu: Vec<PowerMenuItem>,
}
