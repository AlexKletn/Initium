use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserspaceChoice {
    pub name: String,
    pub icon: PathBuf,
}
