use std::io;
use std::ptr::NonNull;

use lvgl_rust::sys;

use crate::DisplayConnection;
use crate::lvgl::with_session;

pub(super) struct AnimationImage<'display> {
    _display: &'display DisplayConnection,
    _pixels: Vec<u32>,
    pixel_ptr: NonNull<u32>,
    pending: Vec<u32>,
    descriptor: Box<sys::lv_image_dsc_t>,
    image: NonNull<sys::lv_obj_t>,
}

impl<'display> AnimationImage<'display> {
    pub fn create(
        display: &'display DisplayConnection,
        width: u32,
        height: u32,
    ) -> io::Result<Self> {
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "animation exceeds LVGL image limits",
            )
        };
        let stride = width
            .checked_mul(4)
            .filter(|stride| *stride > 0 && u16::try_from(*stride).is_ok())
            .ok_or_else(invalid)?;
        let bytes = stride
            .checked_mul(height)
            .filter(|bytes| *bytes > 0 && u16::try_from(height).is_ok())
            .ok_or_else(invalid)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(bytes as usize / 4)
            .map_err(io::Error::other)?;
        pixels.resize(bytes as usize / 4, 0);
        let pixel_ptr = NonNull::new(pixels.as_mut_ptr()).ok_or_else(invalid)?;
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(pixels.len())
            .map_err(io::Error::other)?;
        pending.resize(pixels.len(), 0);
        let mut descriptor = Box::new(sys::lv_image_dsc_t {
            data: pixel_ptr.as_ptr().cast(),
            data_size: bytes,
            ..Default::default()
        });
        descriptor.header.set_magic(sys::LV_IMAGE_HEADER_MAGIC);
        descriptor
            .header
            .set_cf(sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888);
        descriptor.header.set_w(width);
        descriptor.header.set_h(height);
        descriptor.header.set_stride(stride);
        let image = with_session(|_| {
            // SAFETY: The display is live and the descriptor and its backing pixels outlive the widget.
            unsafe {
                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                let image = NonNull::new(sys::lv_image_create(screen.as_ptr()))
                    .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
                sys::lv_image_set_src(
                    image.as_ptr(),
                    std::ptr::from_ref(descriptor.as_ref()).cast(),
                );
                sys::lv_obj_center(image.as_ptr());
                Ok::<_, io::Error>(image)
            }
        })
        .map_err(io::Error::other)??;
        Ok(Self {
            _display: display,
            _pixels: pixels,
            pixel_ptr,
            pending,
            descriptor,
            image,
        })
    }

    pub fn frame(&self) -> (&[u32], usize, usize) {
        (
            &self.pending,
            self.descriptor.header.w() as usize,
            self.descriptor.header.h() as usize,
        )
    }

    pub fn set_offset(&mut self, offset: i32) -> Result<(), crate::Error> {
        with_session(|_| {
            // SAFETY: The image is live and the session is exclusive.
            unsafe { sys::lv_obj_set_style_translate_y(self.image.as_ptr(), offset, 0) };
        })
    }

    pub fn update(
        &mut self,
        read: impl FnOnce(&mut [u32]) -> io::Result<bool>,
    ) -> io::Result<bool> {
        if !read(&mut self.pending)? {
            return Ok(false);
        }

        with_session(|_| {
            // SAFETY: Both disjoint buffers stay allocated and have equal length.
            // LVGL cannot access the display buffer during this exclusive session.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.pending.as_ptr(),
                    self.pixel_ptr.as_ptr(),
                    self.pending.len(),
                );
                sys::lvgl_sys_image_cache_drop(std::ptr::from_ref(self.descriptor.as_ref()).cast());
                sys::lv_obj_invalidate(self.image.as_ptr());
            }
            true
        })
        .map_err(io::Error::other)
    }
}

impl Drop for AnimationImage<'_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The image and buffer live through removal of the widget and cache entry.
            unsafe {
                sys::lv_obj_delete(self.image.as_ptr());
                sys::lvgl_sys_image_cache_drop(std::ptr::from_ref(self.descriptor.as_ref()).cast());
            }
        });
    }
}
