use std::ffi::{CStr, c_char};
use std::io::{self, Write};
use std::sync::{Mutex, MutexGuard};

use lvgl_rust::{Error, Lvgl, LvglAccess, sys};

static LVGL: Mutex<()> = Mutex::new(());

struct Access<'a> {
    _guard: MutexGuard<'a, ()>,
}

// SAFETY: Each Access holds the same mutex exclusively and is borrowed only
// within a single call to initialize or with_session.
unsafe impl LvglAccess for Access<'_> {}

pub fn initialize() -> Result<(), Error> {
    let mut access = lock();

    let _session = lvgl_rust::init(&mut access)?;
    // SAFETY: Registration occurs under exclusive access and the callback is static.
    unsafe { sys::lv_log_register_print_cb(Some(print_error)) };
    Ok(())
}

unsafe extern "C" fn print_error(_level: sys::lv_log_level_t, message: *const c_char) {
    // SAFETY: LVGL provides a NUL-terminated message valid during this callback.
    let message = unsafe { CStr::from_ptr(message) };
    let _ = io::stderr().lock().write_all(message.to_bytes());
}

pub(crate) fn with_session<T>(operation: impl FnOnce(&mut Lvgl) -> T) -> Result<T, Error> {
    let mut access = lock();
    let mut session = Lvgl::enter(&mut access)?;
    Ok(operation(&mut session))
}

fn lock() -> Access<'static> {
    Access {
        _guard: LVGL.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
    }
}

#[cfg(test)]
#[path = "lvgl.tests.rs"]
pub(crate) mod tests;
