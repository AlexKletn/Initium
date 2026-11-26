use std::cell::Cell;
use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::ptr::NonNull;

use lvgl_rust::sys;

use crate::boot_menu::{
    palette,
    style::{place_panel, style_tile},
};
use crate::lvgl::with_session;
use crate::{DisplayConnection, choice_layout::ChoiceLayout};

pub(crate) struct ChoicePanel<'display> {
    _display: &'display DisplayConnection,
    panel: NonNull<sys::lv_obj_t>,
}

impl<'display> ChoicePanel<'display> {
    pub(crate) fn select(&self, selected: usize) -> io::Result<()> {
        with_session(|_| {
            // SAFETY: The panel owns all its tiles in this exclusive LVGL session.
            unsafe {
                let count = sys::lv_obj_get_child_count(self.panel.as_ptr());
                if selected >= count as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unknown menu choice",
                    ));
                }
                for index in 0..count {
                    let tile = sys::lv_obj_get_child(self.panel.as_ptr(), index as i32);
                    if index as usize == selected {
                        sys::lv_obj_add_state(tile, sys::lv_state_t_LV_STATE_CHECKED);
                    } else {
                        sys::lv_obj_remove_state(tile, sys::lv_state_t_LV_STATE_CHECKED);
                    }
                }
                Ok(())
            }
        })
        .map_err(io::Error::other)?
    }

    pub(crate) fn create<'choice>(
        display: &'display DisplayConnection,
        choices: impl Iterator<Item = (&'choice str, &'choice std::path::Path)>,
        layout: ChoiceLayout,
        mut configure: impl FnMut(NonNull<sys::lv_obj_t>) -> io::Result<()>,
    ) -> io::Result<Option<Self>> {
        let entries = choices
            .map(|(name, icon)| {
                let name = CString::new(name).map_err(invalid_text)?;
                let icon = std::path::absolute(icon)?;
                let mut source = b"S:".to_vec();
                source.extend_from_slice(icon.as_os_str().as_bytes());
                let source = CString::new(source).map_err(invalid_text)?;
                Ok((name, source))
            })
            .collect::<io::Result<Vec<_>>>()?;
        if entries.is_empty() {
            return Ok(None);
        }

        with_session(|_| {
            // SAFETY: The display is borrowed for the selector's lifetime.
            // All objects are created under this exclusive LVGL session.
            unsafe {
                for (_, source) in &entries {
                    let mut header = sys::lv_image_header_t::default();
                    if sys::lv_image_decoder_get_info(source.as_ptr().cast(), &mut header)
                        != sys::lv_result_t_LV_RESULT_OK
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("cannot read menu icon {}", source.to_string_lossy()),
                        ));
                    }
                }

                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                let panel = NonNull::new(sys::lv_obj_create(screen.as_ptr()))
                    .ok_or_else(allocation_error)?;
                let (tile_width, tile_height) = layout.tile_size();
                let padding = layout.panel_padding();
                let gap = layout.column_gap();
                let inset = 2 * (padding + 1);
                let max_width =
                    sys::lv_display_get_horizontal_resolution(display.as_ptr()) * 9 / 10;
                let total_width =
                    (entries.len() as i64) * i64::from(tile_width + gap) + i64::from(inset - gap);
                let width = total_width.min(i64::from(max_width)) as i32;
                let per_row = ((width - inset + gap) / (tile_width + gap)).max(1) as usize;
                let rows = entries.len().div_ceil(per_row).min(2) as i32;
                let height = rows * tile_height + (rows - 1) * padding + inset;
                place_panel(panel, width, height, layout.y(height));
                sys::lv_obj_set_style_pad_all(panel.as_ptr(), padding, 0);
                sys::lv_obj_set_style_pad_column(panel.as_ptr(), gap, 0);
                sys::lv_obj_set_style_pad_row(panel.as_ptr(), padding, 0);
                sys::lv_obj_set_flex_flow(
                    panel.as_ptr(),
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP,
                );
                let center = sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER;
                sys::lv_obj_set_flex_align(panel.as_ptr(), center, center, center);
                if matches!(layout, ChoiceLayout::Userspace) {
                    sys::lv_obj_set_style_radius(panel.as_ptr(), 29, 0);
                }
                sys::lv_obj_remove_flag(panel.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
                sys::lv_obj_remove_flag(
                    panel.as_ptr(),
                    sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLL_CHAIN,
                );

                for (name, source) in &entries {
                    if let Err(error) =
                        Self::add_choice(panel, name, source, layout, display.theme().accent_rgb)
                            .and_then(&mut configure)
                    {
                        sys::lv_obj_delete(panel.as_ptr());
                        return Err(error);
                    }
                }

                Ok(Some(Self {
                    _display: display,
                    panel,
                }))
            }
        })
        .map_err(io::Error::other)?
    }

    unsafe fn add_choice(
        panel: NonNull<sys::lv_obj_t>,
        name: &std::ffi::CStr,
        source: &std::ffi::CStr,
        layout: ChoiceLayout,
        accent_rgb: u32,
    ) -> io::Result<NonNull<sys::lv_obj_t>> {
        // SAFETY: The caller guarantees a live parent and exclusive access.
        unsafe {
            let tile =
                NonNull::new(sys::lv_obj_create(panel.as_ptr())).ok_or_else(allocation_error)?;
            let (width, height) = layout.tile_size();
            style_tile(tile, accent_rgb);
            sys::lv_obj_set_size(tile.as_ptr(), width, height);
            if matches!(layout, ChoiceLayout::Userspace) {
                sys::lv_obj_set_style_border_width(tile.as_ptr(), 0, 0);
                sys::lv_obj_set_style_border_width(
                    tile.as_ptr(),
                    2,
                    sys::lv_state_t_LV_STATE_CHECKED,
                );
            }
            sys::lv_obj_set_style_pad_row(tile.as_ptr(), 9, 0);
            sys::lv_obj_set_style_pad_column(tile.as_ptr(), 10, 0);
            sys::lv_obj_set_flex_flow(tile.as_ptr(), layout.flow());
            let center = sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER;
            sys::lv_obj_set_flex_align(tile.as_ptr(), center, center, center);

            let icon =
                NonNull::new(sys::lv_image_create(tile.as_ptr())).ok_or_else(allocation_error)?;
            let label =
                NonNull::new(sys::lv_label_create(tile.as_ptr())).ok_or_else(allocation_error)?;
            sys::lv_obj_remove_flag(icon.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
            sys::lv_obj_remove_flag(label.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
            // LVGL copies file sources and label text before the calls return.
            sys::lv_image_set_src(icon.as_ptr(), source.as_ptr().cast());
            if matches!(layout, ChoiceLayout::Userspace) {
                sys::lv_obj_set_size(icon.as_ptr(), 44, 44);
                sys::lv_image_set_inner_align(
                    icon.as_ptr(),
                    sys::lv_image_align_t_LV_IMAGE_ALIGN_CONTAIN_DOWNSCALE,
                );
            }
            sys::lv_label_set_text(label.as_ptr(), name.as_ptr());
            sys::lv_obj_set_size(label.as_ptr(), layout.label_width(), 40);
            sys::lv_label_set_long_mode(
                label.as_ptr(),
                sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_DOTS,
            );
            sys::lv_obj_set_style_text_align(
                label.as_ptr(),
                sys::lv_text_align_t_LV_TEXT_ALIGN_CENTER,
                0,
            );
            sys::lv_obj_set_style_text_font(
                label.as_ptr(),
                &raw const sys::lv_font_montserrat_32,
                0,
            );
            sys::lv_obj_set_style_text_color(label.as_ptr(), sys::lv_color_hex(palette::TEXT), 0);
            Ok(tile)
        }
    }
}

pub(crate) unsafe fn bind_click(
    tile: NonNull<sys::lv_obj_t>,
    selection: *mut Cell<Option<usize>>,
) -> io::Result<()> {
    // SAFETY: The caller keeps the selection cell alive until the tile is destroyed.
    unsafe {
        sys::lv_obj_add_flag(tile.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
        let callback = sys::lv_obj_add_event_cb(
            tile.as_ptr(),
            Some(record_choice),
            sys::lv_event_code_t_LV_EVENT_CLICKED,
            selection.cast(),
        );
        NonNull::new(callback).ok_or_else(allocation_error)?;
    }
    Ok(())
}

unsafe extern "C" fn record_choice(event: *mut sys::lv_event_t) {
    // SAFETY: LVGL supplies a live tile event and its registered selection cell.
    unsafe {
        let selection = sys::lv_event_get_user_data(event).cast::<Cell<Option<usize>>>();
        let tile = sys::lv_event_get_current_target(event).cast::<sys::lv_obj_t>();
        if let Ok(index) = usize::try_from(sys::lv_obj_get_index(tile)) {
            (*selection).set(Some(index));
        }
    }
}

impl Drop for ChoicePanel<'_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The borrowed display is live and this selector owns the panel.
            unsafe { sys::lv_obj_delete(self.panel.as_ptr()) };
        });
    }
}

fn invalid_text(error: std::ffi::NulError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn allocation_error() -> io::Error {
    io::Error::from(io::ErrorKind::OutOfMemory)
}
