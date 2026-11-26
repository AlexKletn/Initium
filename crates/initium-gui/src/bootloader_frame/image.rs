use std::io;
use std::ptr::NonNull;

use lvgl_rust::sys;

use super::BootloaderFrame;
use crate::DisplayConnection;
use crate::lvgl::with_session;

pub struct BootloaderImage<'display, 'frame> {
    display: &'display DisplayConnection,
    _frame: &'frame BootloaderFrame,
    _descriptor: Box<sys::lv_image_dsc_t>,
    image: NonNull<sys::lv_obj_t>,
}

impl<'display, 'frame> BootloaderImage<'display, 'frame> {
    pub(crate) fn create(
        display: &'display DisplayConnection,
        frame: &'frame BootloaderFrame,
    ) -> io::Result<Self> {
        let (width, height) = frame.size();
        let dimensions = || {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "frame exceeds LVGL image limits",
            )
        };
        let stride = width
            .checked_mul(4)
            .filter(|stride| u16::try_from(*stride).is_ok())
            .ok_or_else(dimensions)?;
        let data_size = stride
            .checked_mul(height)
            .filter(|_| u16::try_from(height).is_ok())
            .ok_or_else(dimensions)?;
        let mut descriptor = Box::new(sys::lv_image_dsc_t {
            data: frame.pixels().as_ptr().cast(),
            data_size,
            ..Default::default()
        });
        descriptor.header.set_magic(sys::LV_IMAGE_HEADER_MAGIC);
        descriptor
            .header
            .set_cf(sys::lv_color_format_t_LV_COLOR_FORMAT_XRGB8888);
        descriptor.header.set_w(width);
        descriptor.header.set_h(height);
        descriptor.header.set_stride(stride);

        let image = with_session(|_| {
            // SAFETY: The display is live and the descriptor and pixels outlive the widget.
            unsafe {
                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                sys::lv_obj_set_style_bg_color(
                    screen.as_ptr(),
                    sys::lv_color_hex(display.theme().background_rgb),
                    0,
                );
                sys::lv_obj_set_style_bg_opa(screen.as_ptr(), 255, 0);
                let image = NonNull::new(sys::lv_image_create(screen.as_ptr()))
                    .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
                sys::lv_image_set_src(
                    image.as_ptr(),
                    std::ptr::from_ref(descriptor.as_ref()).cast(),
                );
                sys::lv_obj_center(image.as_ptr());
                sys::lv_obj_move_to_index(image.as_ptr(), 0);
                Ok::<_, io::Error>(image)
            }
        })
        .map_err(io::Error::other)??;

        Ok(Self {
            display,
            _frame: frame,
            _descriptor: descriptor,
            image,
        })
    }

    pub fn hide_background(&mut self) -> Result<(), crate::Error> {
        with_session(|_| {
            // SAFETY: The image is live and the session is exclusive.
            unsafe { sys::lv_obj_set_style_image_opa(self.image.as_ptr(), 0, 0) };
        })
    }

    pub fn set_offset(&mut self, offset: i32) -> Result<(), crate::Error> {
        with_session(|_| {
            // SAFETY: The image is live and the session is exclusive.
            unsafe { sys::lv_obj_set_style_translate_y(self.image.as_ptr(), offset, 0) };
        })
    }

    pub fn show_brand(&mut self, brand: &str) -> io::Result<()> {
        let brand = std::ffi::CString::new(brand)
            .map_err(|source| io::Error::new(io::ErrorKind::InvalidInput, source))?;
        with_session(|_| {
            // SAFETY: The brand widget belongs to the live image under exclusive LVGL access.
            unsafe {
                let label = NonNull::new(sys::lv_label_create(self.image.as_ptr()))
                    .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
                sys::lv_label_set_text(label.as_ptr(), brand.as_ptr());
                sys::lv_obj_set_style_text_font(
                    label.as_ptr(),
                    &raw const sys::lv_font_montserrat_48,
                    0,
                );
                sys::lv_obj_set_style_text_color(
                    label.as_ptr(),
                    sys::lv_color_hex(self.display.theme().accent_rgb),
                    0,
                );
                sys::lv_obj_center(label.as_ptr());
            }
            Ok(())
        })
        .map_err(io::Error::other)?
    }
}

impl Drop for BootloaderImage<'_, '_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The display, descriptor, and frame are live through widget and cache removal.
            unsafe {
                sys::lv_obj_delete(self.image.as_ptr());
                sys::lvgl_sys_image_cache_drop(
                    std::ptr::from_ref(self._descriptor.as_ref()).cast(),
                );
            }
        });
    }
}
