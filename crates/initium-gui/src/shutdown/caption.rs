use std::ffi::CStr;
use std::io;
use std::ptr::{self, NonNull};

use initium_config::Theme;
use lvgl_rust::sys;
use std::path::Path;

use super::ShutdownAction;
use crate::{DisplayConnection, lvgl::with_session};

pub(super) struct Caption {
    pixels: Option<Vec<u8>>,
    size: [u16; 2],
    label: &'static CStr,
    theme: Theme,
}

impl Caption {
    pub fn load(
        action: ShutdownAction,
        custom: Option<&Path>,
        size: Option<[u16; 2]>,
        theme: Theme,
    ) -> io::Result<Self> {
        let label = match action {
            ShutdownAction::Poweroff => c"Power off",
            ShutdownAction::Reboot => c"Restart",
        };
        let size = size.unwrap_or([0, 0]);
        let pixels = custom.map(std::fs::read).transpose()?;
        if let Some(pixels) = &pixels {
            let stride = u32::from(size[0]) * 4;
            let bytes = stride.checked_mul(u32::from(size[1]));
            if stride == 0
                || stride > u32::from(u16::MAX)
                || size[1] == 0
                || bytes.is_none_or(|bytes| bytes as usize != pixels.len())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "shutdown BGRA image does not match caption_size",
                ));
            }
        }
        Ok(Self {
            pixels,
            size,
            label,
            theme,
        })
    }

    pub fn render_into(
        &self,
        buffer: &mut [u8],
        (width, height): (u16, u16),
        stride: u32,
    ) -> io::Result<()> {
        let size = u32::try_from(buffer.len()).map_err(io::Error::other)?;
        let required = stride.checked_mul(u32::from(height));
        if width == 0
            || height == 0
            || stride < u32::from(width) * 4
            || required.is_none_or(|required| required > size)
            || !buffer.as_ptr().cast::<u32>().is_aligned()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid shutdown buffer",
            ));
        }
        with_session(|_| {
            // SAFETY: The display uses an aligned, exclusive buffer that outlives its deletion.
            unsafe {
                let display =
                    NonNull::new(sys::lv_display_create(i32::from(width), i32::from(height)))
                        .ok_or(io::ErrorKind::OutOfMemory)?;
                sys::lv_display_set_color_format(
                    display.as_ptr(),
                    sys::lv_color_format_t_LV_COLOR_FORMAT_XRGB8888,
                );
                sys::lv_display_set_flush_cb(display.as_ptr(), Some(finish_flush));
                sys::lv_display_set_buffers_with_stride(
                    display.as_ptr(),
                    buffer.as_mut_ptr().cast(),
                    ptr::null_mut(),
                    size,
                    stride,
                    sys::lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_DIRECT,
                );
                let result = self.paint(display.as_ptr());
                sys::lv_display_delete(display.as_ptr());
                result
            }
        })
        .map_err(io::Error::other)?
    }

    pub fn render_on(&self, display: &DisplayConnection) -> io::Result<()> {
        with_session(|_| {
            // SAFETY: The borrowed display stays live under the exclusive LVGL session.
            unsafe { self.paint(display.as_ptr()) }
        })
        .map_err(io::Error::other)?
    }

    unsafe fn paint(&self, display: *mut sys::lv_display_t) -> io::Result<()> {
        // SAFETY: The display is live and exclusive, and the widget is deleted before its data.
        unsafe {
            let screen = NonNull::new(sys::lv_display_get_screen_active(display))
                .ok_or_else(|| io::Error::other("shutdown display has no screen"))?;
            let accent = sys::lv_color_hex(self.theme.accent_rgb);
            sys::lv_obj_set_style_bg_color(
                screen.as_ptr(),
                sys::lv_color_hex(self.theme.background_rgb),
                0,
            );
            sys::lv_obj_set_style_bg_opa(screen.as_ptr(), 255, 0);
            if let Some(pixels) = &self.pixels {
                let [width, height] = self.size.map(u32::from);
                let mut descriptor = sys::lv_image_dsc_t {
                    data: pixels.as_ptr(),
                    data_size: pixels.len() as u32,
                    ..Default::default()
                };
                descriptor.header.set_magic(sys::LV_IMAGE_HEADER_MAGIC);
                descriptor
                    .header
                    .set_cf(sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888);
                descriptor.header.set_w(width);
                descriptor.header.set_h(height);
                descriptor.header.set_stride(width * 4);
                let image = NonNull::new(sys::lv_image_create(screen.as_ptr()))
                    .ok_or(io::ErrorKind::OutOfMemory)?;
                let source = ptr::from_ref(&descriptor).cast();
                sys::lv_image_set_src(image.as_ptr(), source);
                sys::lv_obj_set_style_image_recolor(image.as_ptr(), accent, 0);
                sys::lv_obj_set_style_image_recolor_opa(image.as_ptr(), 255, 0);
                sys::lv_obj_center(image.as_ptr());
                sys::lv_refr_now(display);
                sys::lv_obj_delete(image.as_ptr());
                sys::lvgl_sys_image_cache_drop(source);
            } else {
                let label = NonNull::new(sys::lv_label_create(screen.as_ptr()))
                    .ok_or(io::ErrorKind::OutOfMemory)?;
                sys::lv_label_set_text_static(label.as_ptr(), self.label.as_ptr());
                sys::lv_obj_set_style_text_font(
                    label.as_ptr(),
                    &raw const sys::lv_font_montserrat_48,
                    0,
                );
                sys::lv_obj_set_style_text_color(label.as_ptr(), accent, 0);
                sys::lv_obj_center(label.as_ptr());
                sys::lv_refr_now(display);
                sys::lv_obj_delete(label.as_ptr());
            }
        }
        Ok(())
    }
}

unsafe extern "C" fn finish_flush(
    display: *mut sys::lv_display_t,
    _: *const sys::lv_area_t,
    _: *mut u8,
) {
    // SAFETY: LVGL passes the live display that owns this synchronous flush.
    unsafe { sys::lv_display_flush_ready(display) }
}

#[cfg(test)]
#[path = "caption.tests.rs"]
pub(crate) mod tests;
