use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum HardwareProbe {
    Path {
        path: PathBuf,
        equals: Option<String>,
    },
    Glob {
        glob: String,
    },
}
