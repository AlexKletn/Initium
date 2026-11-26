use std::num::NonZeroU32;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplashOutroConfig {
    pub merge_ms: NonZeroU32,
    pub collapse_ms: NonZeroU32,
    pub core_radius_px: NonZeroU32,
    pub region_radius_px: NonZeroU32,
    pub satellite_rgb_weights: [u8; 3],
}

impl SplashOutroConfig {
    pub fn fits(self, width: u32, height: u32) -> bool {
        let radius = self.region_radius_px.get();
        self.core_radius_px.get() < radius
            && radius <= (width.min(height).saturating_sub(1)) / 2
            && self.satellite_rgb_weights != [0; 3]
    }
}
