use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerCommand {
    pub program: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
}
