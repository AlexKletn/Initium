use std::path::PathBuf;

use serde::Deserialize;

use super::{BootloaderCoverConfig, BootloaderRegionConfig};

#[derive(Debug, Deserialize)]
pub struct BootloaderFrameConfig {
    pub capture_device: Option<PathBuf>,
    pub cover: Option<BootloaderCoverConfig>,
    pub reserved_memory: Option<BootloaderRegionConfig>,
}
