use std::path::PathBuf;

use serde::Deserialize;

use super::{BootloaderFrameConfig, SplashAnimationConfig};

#[derive(Debug, Default, Deserialize)]
pub struct SplashConfig {
    pub brand_text: Option<String>,
    pub intro_frames: Option<PathBuf>,
    pub loop_frames: Option<PathBuf>,
    pub animation: Option<SplashAnimationConfig>,
    pub bootloader_frame: Option<BootloaderFrameConfig>,
}
