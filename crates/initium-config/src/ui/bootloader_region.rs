use std::num::NonZeroU32;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootloaderRegionConfig {
    pub node: String,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}
