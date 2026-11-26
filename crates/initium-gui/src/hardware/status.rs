use std::{fs, io};

use initium_config::{HardwareItem, HardwareProbe};

#[derive(PartialEq, Eq)]
pub(super) enum HardwareStatus {
    Pending,
    Detected,
    Ready,
    Unavailable,
}

impl HardwareStatus {
    pub(super) fn read(item: &HardwareItem) -> Self {
        match item.ready.as_ref().map(matches_probe).transpose() {
            Ok(Some(true)) => return Self::Ready,
            Ok(_) => {}
            Err(_) => return Self::Unavailable,
        }

        match item.detected.as_ref().map(matches_probe).transpose() {
            Ok(Some(true)) => Self::Detected,
            Ok(_) => Self::Pending,
            Err(_) => Self::Unavailable,
        }
    }

    pub(super) fn label(&self) -> &'static std::ffi::CStr {
        match self {
            Self::Pending => c"Pending",
            Self::Detected => c"Detected",
            Self::Ready => c"Ready",
            Self::Unavailable => c"Unavailable",
        }
    }

    pub(super) fn colors(&self) -> (u32, u32) {
        match self {
            Self::Ready => (0xb6e0b9, 0x223429),
            Self::Detected => (0xf0c16a, 0x392d1c),
            Self::Pending => (0xc3b39e, 0x2b2520),
            Self::Unavailable => (0xf2a895, 0x402624),
        }
    }
}

fn matches_probe(probe: &HardwareProbe) -> io::Result<bool> {
    match probe {
        HardwareProbe::Path {
            path,
            equals: Some(expected),
        } => match fs::read_to_string(path) {
            Ok(value) => Ok(value.trim() == expected),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        },
        HardwareProbe::Path { path, equals: None } => path.try_exists(),
        HardwareProbe::Glob { glob } => {
            let paths = glob::glob(glob)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
            for path in paths {
                if path.map_err(io::Error::other)?.try_exists()? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}
