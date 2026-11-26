use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

const SYSTEM_THEME: &str = "/etc/initium/theme.toml";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub background_rgb: u32,
    pub accent_rgb: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background_rgb: 0x000000,
            accent_rgb: 0xf0c16a,
        }
    }
}

impl Theme {
    pub fn load_system() -> io::Result<Self> {
        Self::load(Path::new(SYSTEM_THEME))
    }

    fn load(path: &Path) -> io::Result<Self> {
        let document = match fs::read_to_string(path) {
            Ok(document) => document,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        Self::parse(&document)
    }

    pub fn parse(document: &str) -> io::Result<Self> {
        let theme: Self = toml::from_str(document)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if theme.background_rgb > 0xff_ffff || theme.accent_rgb > 0xff_ffff {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "theme colors must be 24-bit RGB",
            ));
        }
        Ok(theme)
    }
}

#[cfg(test)]
#[path = "theme.tests.rs"]
mod tests;
