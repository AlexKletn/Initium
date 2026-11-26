use serde::Deserialize;

use super::HardwareProbe;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareItem {
    pub name: String,
    pub detected: Option<HardwareProbe>,
    pub ready: Option<HardwareProbe>,
}
