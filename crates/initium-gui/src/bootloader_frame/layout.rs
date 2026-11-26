use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::ptr;

use super::BootloaderFrame;

#[derive(Default)]
#[repr(C)]
pub(super) struct Layout {
    pub width: u32,
    pub height: u32,
    pub x_offset: u32,
    pub y_offset: u32,
    pub stride: u32,
    pub length: u32,
}

impl Layout {
    pub fn read_frame(&self, file: &File, offset: libc::off_t) -> io::Result<BootloaderFrame> {
        let (first, width_bytes, length) = self.visible_range()?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(width_bytes * self.height as usize / 4)
            .map_err(io::Error::other)?;

        // SAFETY: The reader supplies an open device and a validated mapping offset.
        let address = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                offset,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        for row in 0..self.height as usize {
            let offset = first + row * self.stride as usize;
            for column in (0..width_bytes).step_by(4) {
                // SAFETY: visible_range validates every row bound and u32 alignment.
                let pixel = unsafe {
                    address
                        .cast::<u8>()
                        .add(offset + column)
                        .cast::<u32>()
                        .read_volatile()
                };
                pixels.push(pixel);
            }
        }

        // SAFETY: The original address and length identify this call's mapping.
        unsafe { libc::munmap(address, length) };

        Ok(BootloaderFrame::new(self.width, self.height, pixels))
    }

    fn visible_range(&self) -> io::Result<(usize, usize, usize)> {
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid visible framebuffer layout",
            )
        };
        let width = u64::from(self.width) * 4;
        let x = u64::from(self.x_offset) * 4;
        let stride = u64::from(self.stride);
        if self.width == 0 || self.height == 0 || stride % 4 != 0 || x + width > stride {
            return Err(invalid());
        }

        let first = u64::from(self.y_offset) * stride + x;
        let end = u64::from(self.height - 1)
            .checked_mul(stride)
            .and_then(|remaining| first.checked_add(remaining))
            .and_then(|last| last.checked_add(width))
            .ok_or_else(invalid)?;
        if end > u64::from(self.length) || end > isize::MAX as u64 {
            return Err(invalid());
        }

        Ok((first as usize, width as usize, end as usize))
    }
}
