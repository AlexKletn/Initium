use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "backend", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DisplayConfig {
    Fbdev {
        path: PathBuf,
    },
    DrmKms {
        path: PathBuf,
        connector_id: Option<u32>,
    },
}
