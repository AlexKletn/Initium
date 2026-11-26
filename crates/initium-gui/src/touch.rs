use std::fs::File;
use std::io;
use std::os::fd::IntoRawFd;
use std::path::Path;
use std::ptr::NonNull;

use lvgl_rust::sys;

use crate::DisplayConnection;
use crate::lvgl::with_session;

pub struct Touch<'display> {
    _display: &'display DisplayConnection,
    input: NonNull<sys::lv_indev_t>,
}

impl<'display> Touch<'display> {
    pub fn open(display: &'display DisplayConnection, path: &Path) -> io::Result<Self> {
        let device = File::open(path)?;
        let input = with_session(|_| {
            // SAFETY: The exclusive session protects LVGL and evdev takes ownership of the fd.
            unsafe {
                let input = NonNull::new(sys::lv_evdev_create_fd(
                    sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER,
                    device.into_raw_fd(),
                ))
                .ok_or_else(|| io::Error::other("LVGL could not open touchscreen"))?;
                sys::lv_indev_set_display(input.as_ptr(), display.as_ptr());
                Ok::<_, io::Error>(input)
            }
        })
        .map_err(io::Error::other)??;
        Ok(Self {
            _display: display,
            input,
        })
    }
}

impl Drop for Touch<'_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: This touch adapter owns the LVGL input device and the session is exclusive.
            unsafe { sys::lv_indev_delete(self.input.as_ptr()) };
        });
    }
}
