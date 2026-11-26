use std::io;
use std::ptr::NonNull;

use lvgl_rust::sys;

use super::{entry::Entry, level::Level, reader::read_kernel_log};
use crate::DisplayConnection;
use crate::boot_menu::{palette, style::place_panel};
use crate::lvgl::with_session;

pub struct BootLogPanel<'display> {
    _display: &'display DisplayConnection,
    panel: NonNull<sys::lv_obj_t>,
}

impl<'display> BootLogPanel<'display> {
    pub(crate) fn open(display: &'display DisplayConnection) -> io::Result<Self> {
        match read_kernel_log() {
            Ok(entries) if !entries.is_empty() => Self::render(display, entries),
            Ok(_) => Self::render(
                display,
                [Entry {
                    timestamp: c"-".into(),
                    message: c"Kernel log is empty".into(),
                    level: Level::Info,
                }],
            ),
            Err(_) => Self::render(
                display,
                [Entry {
                    timestamp: c"-".into(),
                    message: c"Kernel log unavailable".into(),
                    level: Level::Warning,
                }],
            ),
        }
    }

    pub(crate) fn render(
        display: &'display DisplayConnection,
        entries: impl IntoIterator<Item = Entry>,
    ) -> io::Result<Self> {
        let entries = entries.into_iter().collect::<Vec<_>>();
        with_session(|_| {
            // SAFETY: The display is borrowed and all widgets are created under exclusive access.
            unsafe {
                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                let panel = NonNull::new(sys::lv_obj_create(screen.as_ptr()))
                    .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
                let max_width =
                    sys::lv_display_get_horizontal_resolution(display.as_ptr()) * 9 / 10;
                let height = 34 + entries.len().clamp(1, 12) as i32 * 48;
                place_panel(panel, 2350.min(max_width), height, 128 + height / 2);
                sys::lv_obj_set_style_pad_row(panel.as_ptr(), 0, 0);
                sys::lv_obj_set_flex_flow(panel.as_ptr(), sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN);
                sys::lv_obj_set_scroll_dir(panel.as_ptr(), sys::lv_dir_t_LV_DIR_VER);
                sys::lv_obj_remove_flag(
                    panel.as_ptr(),
                    sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLL_CHAIN,
                );

                let mut last = None;
                for (index, entry) in entries.iter().enumerate() {
                    match add_row(panel, entry, index, 2350.min(max_width) - 34) {
                        Ok(row) => last = Some(row),
                        Err(error) => {
                            sys::lv_obj_delete(panel.as_ptr());
                            return Err(error);
                        }
                    }
                }
                if let Some(label) = last {
                    sys::lv_obj_update_layout(panel.as_ptr());
                    sys::lv_obj_scroll_to_view(label.as_ptr(), false);
                }

                Ok(Self {
                    _display: display,
                    panel,
                })
            }
        })
        .map_err(io::Error::other)?
    }
}

impl Drop for BootLogPanel<'_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The panel owns these widgets and the display is still live.
            unsafe { sys::lv_obj_delete(self.panel.as_ptr()) };
        });
    }
}

unsafe fn add_row(
    panel: NonNull<sys::lv_obj_t>,
    entry: &Entry,
    index: usize,
    width: i32,
) -> io::Result<NonNull<sys::lv_obj_t>> {
    // SAFETY: The caller owns the live parent and all operations occur in an exclusive session.
    unsafe {
        let row =
            NonNull::new(sys::lv_obj_create(panel.as_ptr())).ok_or(io::ErrorKind::OutOfMemory)?;
        sys::lv_obj_remove_style_all(row.as_ptr());
        sys::lv_obj_remove_flag(row.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLLABLE);
        sys::lv_obj_remove_flag(row.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
        sys::lv_obj_set_size(row.as_ptr(), sys::lv_pct(100), 48);
        if index.is_multiple_of(2) {
            sys::lv_obj_set_style_bg_color(row.as_ptr(), sys::lv_color_hex(palette::ROW), 0);
            sys::lv_obj_set_style_bg_opa(row.as_ptr(), 255, 0);
            sys::lv_obj_set_style_radius(row.as_ptr(), 8, 0);
        }

        let (mark, color) = entry.level.badge();
        for (text, x, size, color, align) in [
            (
                entry.timestamp.as_c_str(),
                12,
                125,
                palette::MUTED,
                sys::lv_text_align_t_LV_TEXT_ALIGN_RIGHT,
            ),
            (
                mark,
                156,
                40,
                color,
                sys::lv_text_align_t_LV_TEXT_ALIGN_CENTER,
            ),
            (
                entry.message.as_c_str(),
                214,
                (width - 230).max(1),
                palette::TEXT,
                sys::lv_text_align_t_LV_TEXT_ALIGN_LEFT,
            ),
        ] {
            let label = NonNull::new(sys::lv_label_create(row.as_ptr()))
                .ok_or(io::ErrorKind::OutOfMemory)?;
            sys::lv_obj_set_width(label.as_ptr(), size);
            sys::lv_label_set_long_mode(
                label.as_ptr(),
                sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_CLIP,
            );
            sys::lv_obj_align(label.as_ptr(), sys::lv_align_t_LV_ALIGN_LEFT_MID, x, 0);
            sys::lv_obj_set_style_text_align(label.as_ptr(), align, 0);
            sys::lv_obj_set_style_text_font(
                label.as_ptr(),
                &raw const sys::lv_font_montserrat_24,
                0,
            );
            sys::lv_obj_set_style_text_color(label.as_ptr(), sys::lv_color_hex(color), 0);
            sys::lv_label_set_text(label.as_ptr(), text.as_ptr());
        }
        Ok(row)
    }
}
