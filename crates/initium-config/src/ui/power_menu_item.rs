use std::path::PathBuf;

use serde::Deserialize;

use super::PowerCommand;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerMenuItem {
    pub name: String,
    pub icon: PathBuf,
    pub command: PowerCommand,
}
