use std::ffi::CStr;

pub(super) enum Level {
    Error,
    Warning,
    Info,
}

impl Level {
    pub fn from_priority(priority: u8) -> Self {
        match priority & 7 {
            0..=3 => Self::Error,
            4 => Self::Warning,
            _ => Self::Info,
        }
    }

    pub fn badge(&self) -> (&'static CStr, u32) {
        match self {
            Self::Error => (c"X", 0xf2a895),
            Self::Warning => (c"!", 0xf0c16a),
            Self::Info => (c"•", 0x927d62),
        }
    }
}
