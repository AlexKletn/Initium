use std::ffi::CString;
use std::io;
use std::ptr::NonNull;

use initium_config::HardwareItem;
use lvgl_rust::sys;

use super::status::HardwareStatus;
use crate::DisplayConnection;
use crate::boot_menu::{palette, style::place_panel};
use crate::lvgl::with_session;

pub struct HardwarePanel<'display, 'config> {
    _display: &'display DisplayConnection,
    panel: NonNull<sys::lv_obj_t>,
    labels: Vec<NonNull<sys::lv_obj_t>>,
    progress: NonNull<sys::lv_obj_t>,
    summary: NonNull<sys::lv_obj_t>,
    items: &'config [HardwareItem],
}

impl<'display, 'config> HardwarePanel<'display, 'config> {
    pub(crate) fn create(
        display: &'display DisplayConnection,
        items: &'config [HardwareItem],
    ) -> io::Result<Option<Self>> {
        if items.is_empty() {
            return Ok(None);
        }

        let names = items
            .iter()
            .map(|item| CString::new(item.name.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let (states, ready) = read_statuses(items);
        let panel = with_session(|_| {
            // SAFETY: The connected display is live and the LVGL session is exclusive.
            unsafe {
                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                let panel = NonNull::new(sys::lv_obj_create(screen.as_ptr()))
                    .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
                let max_width =
                    sys::lv_display_get_horizontal_resolution(display.as_ptr()) * 9 / 10;
                let height = 88 + items.len().min(12) as i32 * 48;
                place_panel(panel, 1152.min(max_width), height, 128 + height / 2);
                sys::lv_obj_set_style_pad_top(panel.as_ptr(), 70, 0);
                sys::lv_obj_set_style_pad_row(panel.as_ptr(), 0, 0);
                sys::lv_obj_remove_flag(
                    panel.as_ptr(),
                    sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLL_CHAIN,
                );
                sys::lv_obj_set_flex_flow(panel.as_ptr(), sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN);

                let mut labels = Vec::with_capacity(items.len());
                for (index, (name, state)) in names.iter().zip(&states).enumerate() {
                    match add_row(panel, name, state, index) {
                        Ok(status) => labels.push(status),
                        Err(error) => {
                            sys::lv_obj_delete(panel.as_ptr());
                            return Err(error);
                        }
                    }
                }

                let progress = NonNull::new(sys::lv_bar_create(panel.as_ptr()));
                let summary = NonNull::new(sys::lv_label_create(panel.as_ptr()));
                let (Some(progress), Some(summary)) = (progress, summary) else {
                    sys::lv_obj_delete(panel.as_ptr());
                    return Err(io::ErrorKind::OutOfMemory.into());
                };
                sys::lv_obj_add_flag(
                    progress.as_ptr(),
                    sys::lv_obj_flag_t_LV_OBJ_FLAG_IGNORE_LAYOUT,
                );
                sys::lv_obj_set_size(progress.as_ptr(), sys::lv_pct(70), 6);
                sys::lv_obj_align(progress.as_ptr(), sys::lv_align_t_LV_ALIGN_TOP_LEFT, 0, -33);
                sys::lv_obj_set_style_bg_color(
                    progress.as_ptr(),
                    sys::lv_color_hex(palette::SELECTED),
                    0,
                );
                sys::lv_obj_set_style_bg_color(
                    progress.as_ptr(),
                    sys::lv_color_hex(display.theme().accent_rgb),
                    sys::lv_part_t_LV_PART_INDICATOR,
                );
                sys::lv_bar_set_range(
                    progress.as_ptr(),
                    0,
                    items.len().min(i32::MAX as usize) as i32,
                );

                sys::lv_obj_add_flag(
                    summary.as_ptr(),
                    sys::lv_obj_flag_t_LV_OBJ_FLAG_IGNORE_LAYOUT,
                );
                sys::lv_obj_set_style_text_font(
                    summary.as_ptr(),
                    &raw const sys::lv_font_montserrat_24,
                    0,
                );
                sys::lv_obj_set_style_text_color(
                    summary.as_ptr(),
                    sys::lv_color_hex(palette::MUTED),
                    0,
                );
                sys::lv_obj_align(summary.as_ptr(), sys::lv_align_t_LV_ALIGN_TOP_RIGHT, 0, -42);
                set_progress(progress, summary, ready, items.len());

                Ok(Self {
                    _display: display,
                    panel,
                    labels,
                    progress,
                    summary,
                    items,
                })
            }
        })
        .map_err(io::Error::other)??;

        Ok(Some(panel))
    }

    pub fn update(&self) -> io::Result<()> {
        let (states, ready) = read_statuses(self.items);
        with_session(|_| {
            for (label, state) in self.labels.iter().zip(&states) {
                // SAFETY: This panel owns the labels and LVGL access is exclusive.
                unsafe { set_status(*label, state) };
            }
            // SAFETY: The progress widgets belong to the panel in this exclusive session.
            unsafe { set_progress(self.progress, self.summary, ready, self.items.len()) };
        })
        .map_err(io::Error::other)
    }
}

impl Drop for HardwarePanel<'_, '_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The display is borrowed and this panel exclusively owns its widgets.
            unsafe { sys::lv_obj_delete(self.panel.as_ptr()) };
        });
    }
}

fn read_statuses(items: &[HardwareItem]) -> (Vec<HardwareStatus>, usize) {
    let states = items.iter().map(HardwareStatus::read).collect::<Vec<_>>();
    let ready = states
        .iter()
        .filter(|state| **state == HardwareStatus::Ready)
        .count();
    (states, ready)
}

unsafe fn add_row(
    panel: NonNull<sys::lv_obj_t>,
    name: &std::ffi::CStr,
    state: &HardwareStatus,
    index: usize,
) -> io::Result<NonNull<sys::lv_obj_t>> {
    // SAFETY: The caller owns the live panel and LVGL access is exclusive.
    unsafe {
        let row =
            NonNull::new(sys::lv_obj_create(panel.as_ptr())).ok_or(io::ErrorKind::OutOfMemory)?;
        sys::lv_obj_remove_style_all(row.as_ptr());
        sys::lv_obj_set_size(row.as_ptr(), sys::lv_pct(100), 48);
        sys::lv_obj_remove_flag(row.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLLABLE);
        sys::lv_obj_remove_flag(row.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
        if index.is_multiple_of(2) {
            sys::lv_obj_set_style_bg_color(row.as_ptr(), sys::lv_color_hex(palette::ROW), 0);
            sys::lv_obj_set_style_bg_opa(row.as_ptr(), 255, 0);
            sys::lv_obj_set_style_radius(row.as_ptr(), 8, 0);
        }

        let device =
            NonNull::new(sys::lv_label_create(row.as_ptr())).ok_or(io::ErrorKind::OutOfMemory)?;
        sys::lv_obj_set_width(device.as_ptr(), sys::lv_pct(68));
        sys::lv_label_set_long_mode(
            device.as_ptr(),
            sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_DOTS,
        );
        sys::lv_obj_set_style_text_font(device.as_ptr(), &raw const sys::lv_font_montserrat_32, 0);
        sys::lv_obj_set_style_text_color(device.as_ptr(), sys::lv_color_hex(palette::TEXT), 0);
        sys::lv_obj_align(device.as_ptr(), sys::lv_align_t_LV_ALIGN_LEFT_MID, 12, 0);
        sys::lv_label_set_text(device.as_ptr(), name.as_ptr());

        let status =
            NonNull::new(sys::lv_label_create(row.as_ptr())).ok_or(io::ErrorKind::OutOfMemory)?;
        sys::lv_obj_set_width(status.as_ptr(), 176);
        sys::lv_obj_set_style_pad_ver(status.as_ptr(), 5, 0);
        sys::lv_obj_set_style_radius(status.as_ptr(), 8, 0);
        sys::lv_obj_set_style_bg_opa(status.as_ptr(), 255, 0);
        sys::lv_obj_set_style_text_align(
            status.as_ptr(),
            sys::lv_text_align_t_LV_TEXT_ALIGN_CENTER,
            0,
        );
        sys::lv_obj_set_style_text_font(status.as_ptr(), &raw const sys::lv_font_montserrat_24, 0);
        sys::lv_obj_align(status.as_ptr(), sys::lv_align_t_LV_ALIGN_RIGHT_MID, -8, 0);
        set_status(status, state);
        Ok(status)
    }
}

unsafe fn set_status(label: NonNull<sys::lv_obj_t>, status: &HardwareStatus) {
    let (text, background) = status.colors();
    // SAFETY: The caller owns the live label in this exclusive session.
    unsafe {
        sys::lv_label_set_text(label.as_ptr(), status.label().as_ptr());
        sys::lv_obj_set_style_text_color(label.as_ptr(), sys::lv_color_hex(text), 0);
        sys::lv_obj_set_style_bg_color(label.as_ptr(), sys::lv_color_hex(background), 0);
    }
}

unsafe fn set_progress(
    bar: NonNull<sys::lv_obj_t>,
    label: NonNull<sys::lv_obj_t>,
    ready: usize,
    total: usize,
) {
    let text = format!("{ready} / {total}\0");
    // SAFETY: The caller supplies live objects under exclusive access. LVGL copies the text.
    unsafe {
        sys::lv_bar_set_value(bar.as_ptr(), ready.min(i32::MAX as usize) as i32, false);
        sys::lv_label_set_text(label.as_ptr(), text.as_ptr().cast());
    }
}
