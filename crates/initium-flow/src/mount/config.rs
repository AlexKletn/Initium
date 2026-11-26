use std::path::PathBuf;

use serde::Deserialize;

use super::flag::MountFlag;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MountConfig {
    pub source: Option<String>,
    pub target: PathBuf,
    pub filesystem: String,
    #[serde(default)]
    pub flags: Vec<MountFlag>,
    pub options: Option<String>,
}
