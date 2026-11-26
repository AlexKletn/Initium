use std::num::NonZeroU32;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplashAnimationConfig {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub intro_fps: NonZeroU32,
    pub loop_fps: NonZeroU32,

    pub outro: Option<super::SplashOutroConfig>,
}
