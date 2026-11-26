use std::cell::Cell;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::ptr::{self, NonNull};

use lvgl_rust::sys;

#[derive(Default)]
#[repr(C)]
struct Mapping {
    offset: u64,
    width: u32,
    height: u32,
    stride: u32,
    length: u32,
    handle: u32,
}

unsafe extern "C" {
    fn initium_drm_front_mapping(
        fd: libc::c_int,
        framebuffer: u32,
        out: *mut Mapping,
    ) -> libc::c_int;
    fn drmCloseBufferHandle(fd: libc::c_int, handle: u32) -> libc::c_int;
}

pub(super) struct DrmFrontBuffer {
    address: NonNull<libc::c_void>,
    mapping: Mapping,
    staging: Vec<u32>,
    dirty: Cell<Option<sys::lv_area_t>>,
}

impl DrmFrontBuffer {
    pub fn map(fd: BorrowedFd<'_>, framebuffer: u32) -> io::Result<Self> {
        let mut mapping = Mapping::default();
        // SAFETY: The DRM descriptor and matching C output layout are live during the call.
        let result =
            unsafe { initium_drm_front_mapping(fd.as_raw_fd(), framebuffer, &mut mapping) };
        if result < 0 {
            return Err(io::Error::from_raw_os_error(-result));
        }
        let address = libc::off_t::try_from(mapping.offset).map(|offset| {
            // SAFETY: The offset and size describe the verified, linear DRM buffer.
            unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    mapping.length as usize,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    fd.as_raw_fd(),
                    offset,
                )
            }
        });
        let mapping_error = io::Error::last_os_error();
        // SAFETY: GETFB2 returned a fresh handle; the live mapping holds its own reference.
        unsafe { drmCloseBufferHandle(fd.as_raw_fd(), mapping.handle) };
        let address = address.map_err(io::Error::other)?;
        if address == libc::MAP_FAILED {
            return Err(mapping_error);
        }
        let Some(address) = NonNull::new(address) else {
            // SAFETY: The zero address and length identify the just-created mapping.
            unsafe { libc::munmap(address, mapping.length as usize) };
            return Err(io::Error::other("DRM returned a null framebuffer mapping"));
        };
        let mut staging = vec![0; (mapping.length as usize).div_ceil(4)];
        // SAFETY: Both buffers cover the verified mapping length and do not overlap.
        unsafe {
            ptr::copy_nonoverlapping(
                address.as_ptr().cast::<u8>(),
                staging.as_mut_ptr().cast::<u8>(),
                mapping.length as usize,
            );
        }
        Ok(Self {
            address,
            mapping,
            staging,
            dirty: Cell::new(None),
        })
    }

    pub unsafe fn attach(&mut self, display: NonNull<sys::lv_display_t>) -> io::Result<()> {
        // SAFETY: The caller retains the display and mapping under exclusive LVGL access.
        unsafe {
            if sys::lv_display_get_horizontal_resolution(display.as_ptr())
                != self.mapping.width as i32
                || sys::lv_display_get_vertical_resolution(display.as_ptr())
                    != self.mapping.height as i32
            {
                return Err(io::Error::other(
                    "front buffer does not match display resolution",
                ));
            }
            sys::lv_display_set_flush_wait_cb(display.as_ptr(), None);
            sys::lv_display_flush_ready(display.as_ptr());
            sys::lv_display_set_user_data(display.as_ptr(), ptr::from_mut(self).cast());
            sys::lv_display_set_flush_cb(display.as_ptr(), Some(front_buffer_flush));
            sys::lv_display_set_buffers_with_stride(
                display.as_ptr(),
                self.staging.as_mut_ptr().cast(),
                ptr::null_mut(),
                self.mapping.length,
                self.mapping.stride,
                sys::lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_DIRECT,
            );
        }
        Ok(())
    }
    pub fn has_pending_pixels(&self) -> bool {
        self.dirty.get().is_some()
    }

    pub fn publish(&self) {
        let Some(area) = self.dirty.take() else {
            return;
        };
        let left = area.x1.max(0);
        let right = area.x2.min(self.mapping.width as i32 - 1);
        if left > right {
            return;
        }
        let first_row = area.y1.max(0);
        let last_row = area.y2.min(self.mapping.height as i32 - 1);
        let row_bytes = (right - left + 1) as usize * 4;
        for row in first_row..=last_row {
            let offset = row as usize * self.mapping.stride as usize + left as usize * 4;
            // SAFETY: The clipped row is inside both equally sized, disjoint buffers.
            unsafe {
                ptr::copy_nonoverlapping(
                    self.staging.as_ptr().cast::<u8>().add(offset),
                    self.address.as_ptr().cast::<u8>().add(offset),
                    row_bytes,
                );
            }
        }
    }
}

impl Drop for DrmFrontBuffer {
    fn drop(&mut self) {
        // SAFETY: The mapping remains owned until the LVGL display is destroyed.
        unsafe { libc::munmap(self.address.as_ptr(), self.mapping.length as usize) };
    }
}

unsafe extern "C" fn front_buffer_flush(
    display: *mut sys::lv_display_t,
    area: *const sys::lv_area_t,
    _pixels: *mut u8,
) {
    // SAFETY: LVGL supplies the flush area, and the attached buffer outlives the display.
    unsafe {
        let front = &*sys::lv_display_get_user_data(display).cast::<DrmFrontBuffer>();
        let area = *area;
        let dirty = front.dirty.get().map_or(area, |previous| sys::lv_area_t {
            x1: previous.x1.min(area.x1),
            y1: previous.y1.min(area.y1),
            x2: previous.x2.max(area.x2),
            y2: previous.y2.max(area.y2),
        });
        front.dirty.set(Some(dirty));
        sys::lv_display_flush_ready(display);
    };
}
