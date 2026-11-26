use std::{cell::Cell, io, ptr::NonNull};

use lvgl_rust::sys;

use super::{action::Action, palette, style::style_tile};
use crate::{DisplayConnection, lvgl::with_session};

pub(super) struct Controls<'display> {
    _display: &'display DisplayConnection,
    root: NonNull<sys::lv_obj_t>,
    clicked: Box<Cell<Option<usize>>>,
    actions: Vec<Action>,
}

impl<'display> Controls<'display> {
    pub fn create(display: &'display DisplayConnection, actions: Vec<Action>) -> io::Result<Self> {
        let mut clicked = Box::new(Cell::new(None));
        let data = std::ptr::from_mut(clicked.as_mut()).cast();
        with_session(|_| {
            // SAFETY: The display and callback state remain live through controls removal.
            unsafe {
                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                let root = NonNull::new(sys::lv_obj_create(screen.as_ptr())).ok_or_else(oom)?;
                sys::lv_obj_remove_style_all(root.as_ptr());
                sys::lv_obj_set_size(root.as_ptr(), 640, 192);
                sys::lv_obj_center(root.as_ptr());
                sys::lv_obj_remove_flag(root.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLLABLE);
                sys::lv_obj_remove_flag(root.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
                sys::lv_obj_add_flag(root.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_HIDDEN);

                for action in &actions {
                    if let Err(error) = add_button(root, *action, data, display.theme().accent_rgb)
                    {
                        sys::lv_obj_delete(root.as_ptr());
                        return Err(error);
                    }
                }
                Ok(Self {
                    _display: display,
                    root,
                    clicked,
                    actions,
                })
            }
        })
        .map_err(io::Error::other)?
    }

    pub fn take(&self) -> Option<Action> {
        self.clicked
            .take()
            .and_then(|index| self.actions.get(index).copied())
    }

    pub fn show(&self, visible: bool) -> io::Result<()> {
        with_session(|_| {
            // SAFETY: The owner retains the root in this exclusive session.
            unsafe {
                if visible {
                    sys::lv_obj_remove_flag(
                        self.root.as_ptr(),
                        sys::lv_obj_flag_t_LV_OBJ_FLAG_HIDDEN,
                    );
                } else {
                    sys::lv_obj_add_flag(self.root.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_HIDDEN);
                }
            }
        })
        .map_err(io::Error::other)
    }

    pub fn select(&self, selected: Option<Action>) -> io::Result<()> {
        with_session(|_| {
            // SAFETY: The stored actions are in the same order as the live tiles.
            unsafe {
                for (index, action) in self.actions.iter().enumerate() {
                    let tile = sys::lv_obj_get_child(self.root.as_ptr(), index as i32);
                    if selected == Some(*action) {
                        sys::lv_obj_add_state(tile, sys::lv_state_t_LV_STATE_CHECKED);
                    } else {
                        sys::lv_obj_remove_state(tile, sys::lv_state_t_LV_STATE_CHECKED);
                    }
                }
            }
        })
        .map_err(io::Error::other)
    }
}

impl Drop for Controls<'_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The root owns its tiles and the callback state is still live.
            unsafe { sys::lv_obj_delete(self.root.as_ptr()) };
        });
    }
}

unsafe fn add_button(
    root: NonNull<sys::lv_obj_t>,
    action: Action,
    data: *mut std::ffi::c_void,
    accent_rgb: u32,
) -> io::Result<()> {
    // SAFETY: The caller owns the live root in an exclusive LVGL session.
    unsafe {
        let button = NonNull::new(sys::lv_button_create(root.as_ptr())).ok_or_else(oom)?;
        style_tile(button, accent_rgb);
        sys::lv_obj_set_size(button.as_ptr(), 192, 192);
        sys::lv_obj_align(
            button.as_ptr(),
            sys::lv_align_t_LV_ALIGN_CENTER,
            action.x(),
            0,
        );

        let icon = NonNull::new(sys::lv_label_create(button.as_ptr())).ok_or_else(oom)?;
        sys::lv_label_set_text(icon.as_ptr(), action.icon().as_ptr());
        sys::lv_obj_set_style_text_font(icon.as_ptr(), &raw const sys::lv_font_montserrat_48, 0);
        sys::lv_obj_set_style_text_color(icon.as_ptr(), sys::lv_color_hex(accent_rgb), 0);
        sys::lv_obj_align(icon.as_ptr(), sys::lv_align_t_LV_ALIGN_CENTER, 0, -22);

        let label = NonNull::new(sys::lv_label_create(button.as_ptr())).ok_or_else(oom)?;
        sys::lv_label_set_text(label.as_ptr(), action.label().as_ptr());
        sys::lv_obj_set_style_text_font(label.as_ptr(), &raw const sys::lv_font_montserrat_32, 0);
        sys::lv_obj_set_style_text_color(label.as_ptr(), sys::lv_color_hex(palette::TEXT), 0);
        sys::lv_obj_align(label.as_ptr(), sys::lv_align_t_LV_ALIGN_CENTER, 0, 39);
        if sys::lv_obj_add_event_cb(
            button.as_ptr(),
            Some(on_click),
            sys::lv_event_code_t_LV_EVENT_CLICKED,
            data,
        )
        .is_null()
        {
            return Err(oom());
        }
    }
    Ok(())
}

unsafe extern "C" fn on_click(event: *mut sys::lv_event_t) {
    // SAFETY: LVGL supplies a live event and registered state until controls are removed.
    unsafe {
        let clicked = sys::lv_event_get_user_data(event).cast::<Cell<Option<usize>>>();
        let button = sys::lv_event_get_current_target(event).cast::<sys::lv_obj_t>();
        if let Ok(index) = usize::try_from(sys::lv_obj_get_index(button)) {
            (*clicked).set(Some(index));
        }
    }
}

fn oom() -> io::Error {
    io::Error::from(io::ErrorKind::OutOfMemory)
}
