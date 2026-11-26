use std::io;

use initium_config::DisplayConfig;

use super::{DisplayConnection, DisplayPort, DrmKms, Fbdev};

pub fn connect_display(config: &DisplayConfig) -> io::Result<DisplayConnection> {
    match config {
        DisplayConfig::Fbdev { path } => Fbdev::new(path).connect(),
        DisplayConfig::DrmKms { path, connector_id } => DrmKms::new(path, *connector_id).connect(),
    }
}
