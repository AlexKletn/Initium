use std::ptr::NonNull;

use lvgl_rust::sys;

use super::palette;

pub(crate) unsafe fn place_panel(panel: NonNull<sys::lv_obj_t>, width: i32, height: i32, y: i32) {
    // SAFETY: The caller provides a live panel in an exclusive LVGL session.
    unsafe {
        sys::lv_obj_set_size(panel.as_ptr(), width, height);
        sys::lv_obj_align(panel.as_ptr(), sys::lv_align_t_LV_ALIGN_CENTER, 0, y);
        sys::lv_obj_set_style_pad_all(panel.as_ptr(), 16, 0);
        sys::lv_obj_set_style_radius(panel.as_ptr(), 37, 0);
        sys::lv_obj_set_style_bg_opa(panel.as_ptr(), 255, 0);
        sys::lv_obj_set_style_bg_color(panel.as_ptr(), sys::lv_color_hex(palette::PANEL), 0);
        sys::lv_obj_set_style_border_color(panel.as_ptr(), sys::lv_color_hex(palette::BORDER), 0);
        sys::lv_obj_set_style_border_width(panel.as_ptr(), 1, 0);
    }
}

pub(crate) unsafe fn style_tile(tile: NonNull<sys::lv_obj_t>, accent_rgb: u32) {
    use super::palette::*;

    // SAFETY: The caller provides a live tile in an exclusive LVGL session.
    unsafe {
        sys::lv_obj_remove_style_all(tile.as_ptr());
        sys::lv_obj_set_style_pad_all(tile.as_ptr(), 0, 0);
        sys::lv_obj_set_style_radius(tile.as_ptr(), 20, 0);
        sys::lv_obj_set_style_bg_opa(tile.as_ptr(), 255, 0);
        sys::lv_obj_set_style_border_width(tile.as_ptr(), 2, 0);
        for (state, fill, border) in [
            (0, TILE, BORDER),
            (sys::lv_state_t_LV_STATE_CHECKED, SELECTED, accent_rgb),
            (sys::lv_state_t_LV_STATE_PRESSED, PRESSED, accent_rgb),
            (
                sys::lv_state_t_LV_STATE_PRESSED | sys::lv_state_t_LV_STATE_CHECKED,
                PRESSED,
                accent_rgb,
            ),
        ] {
            sys::lv_obj_set_style_bg_color(tile.as_ptr(), sys::lv_color_hex(fill), state);
            sys::lv_obj_set_style_border_color(tile.as_ptr(), sys::lv_color_hex(border), state);
        }
    }
}
