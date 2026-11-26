use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

use super::frame::BootloaderFrame;

use super::layout::Layout;

unsafe extern "C" {
    fn initium_framebuffer_layout(fd: libc::c_int, out: *mut Layout) -> libc::c_int;
}

pub(super) fn capture(path: &Path) -> io::Result<BootloaderFrame> {
    let file = File::open(path)?;
    let mut layout = Layout::default();
    // SAFETY: The descriptor is open and the output matches the C layout.
    let result = unsafe { initium_framebuffer_layout(file.as_raw_fd(), &mut layout) };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(-result));
    }

    layout.read_frame(&file, 0)
}
