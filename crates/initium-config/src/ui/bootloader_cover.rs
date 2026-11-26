use std::num::NonZeroU32;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BootloaderCoverConfig {
    pub reference_width: NonZeroU32,
    pub reference_height: NonZeroU32,
    pub x: u32,
    pub y: u32,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub rgb: [u8; 3],
}
