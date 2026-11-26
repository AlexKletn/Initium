use std::ffi::CStr;

use lvgl_rust::sys;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Journal,
    Hardware,
    Power,
}

impl Action {
    pub fn label(self) -> &'static CStr {
        match self {
            Self::Journal => c"Boot log",
            Self::Hardware => c"Hardware",
            Self::Power => c"Power",
        }
    }

    pub fn icon(self) -> &'static CStr {
        let bytes = match self {
            Self::Journal => sys::LV_SYMBOL_FILE,
            Self::Hardware => sys::LV_SYMBOL_SETTINGS,
            Self::Power => sys::LV_SYMBOL_POWER,
        };
        CStr::from_bytes_with_nul(bytes).unwrap_or(c"")
    }

    pub fn x(self) -> i32 {
        match self {
            Self::Journal => -224,
            Self::Hardware => 0,
            Self::Power => 224,
        }
    }
}
