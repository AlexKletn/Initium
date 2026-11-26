use std::io;
use std::path::PathBuf;

use lvgl_rust::sys;

use super::{DisplayConnection, DisplayPort};

pub struct Fbdev {
    path: PathBuf,
}

impl Fbdev {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl DisplayPort for Fbdev {
    type Connection = DisplayConnection;

    fn connect(&self) -> io::Result<Self::Connection> {
        DisplayConnection::open(&self.path, sys::lv_linux_fbdev_create, |display, file| {
            // SAFETY: The session is exclusive, and file is a live C string.
            unsafe { sys::lv_linux_fbdev_set_file(display.as_ptr(), file.as_ptr()) }
        })
    }
}
