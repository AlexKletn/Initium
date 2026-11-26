use std::cell::Cell;
use std::io;
use std::ptr::NonNull;
use std::time::{Duration, Instant};

use lvgl_rust::sys;

use crate::{DisplayConnection, lvgl::with_session};

const RADIUS: i32 = 115;
const DOUBLE_TAP: Duration = Duration::from_millis(400);

#[derive(Default)]
struct TapState {
    first: Cell<Option<Instant>>,
    double_tap: Cell<bool>,
}

pub(super) struct LogoTap<'display> {
    _display: &'display DisplayConnection,
    target: NonNull<sys::lv_obj_t>,
    screen: NonNull<sys::lv_obj_t>,
    reset: NonNull<sys::lv_event_dsc_t>,
    state: Box<TapState>,
}

impl<'display> LogoTap<'display> {
    pub fn create(display: &'display DisplayConnection) -> io::Result<Self> {
        let mut state = Box::new(TapState::default());
        let data = std::ptr::from_mut(state.as_mut()).cast();
        let (target, screen, reset) = with_session(|_| {
            // SAFETY: The display is live and state outlives the registered callback.
            unsafe {
                let screen = NonNull::new(sys::lv_display_get_screen_active(display.as_ptr()))
                    .ok_or_else(|| io::Error::other("display has no active screen"))?;
                let target = NonNull::new(sys::lv_obj_create(screen.as_ptr()))
                    .ok_or(io::ErrorKind::OutOfMemory)?;
                sys::lv_obj_remove_style_all(target.as_ptr());
                sys::lv_obj_set_size(target.as_ptr(), 2 * RADIUS + 1, 2 * RADIUS + 1);
                sys::lv_obj_center(target.as_ptr());
                sys::lv_obj_remove_flag(target.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLLABLE);
                sys::lv_obj_remove_flag(
                    target.as_ptr(),
                    sys::lv_obj_flag_t_LV_OBJ_FLAG_EVENT_BUBBLE,
                );
                sys::lv_obj_add_flag(target.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_ADV_HITTEST);
                sys::lv_obj_add_flag(target.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
                for (event, callback) in [
                    (
                        sys::lv_event_code_t_LV_EVENT_HIT_TEST,
                        hit_test as unsafe extern "C" fn(_),
                    ),
                    (sys::lv_event_code_t_LV_EVENT_CLICKED, click),
                ] {
                    if sys::lv_obj_add_event_cb(target.as_ptr(), Some(callback), event, data)
                        .is_null()
                    {
                        sys::lv_obj_delete(target.as_ptr());
                        return Err(io::Error::from(io::ErrorKind::OutOfMemory));
                    }
                }
                let reset = NonNull::new(sys::lv_obj_add_event_cb(
                    screen.as_ptr(),
                    Some(reset_tap),
                    sys::lv_event_code_t_LV_EVENT_CLICKED,
                    data,
                ));
                let Some(reset) = reset else {
                    sys::lv_obj_delete(target.as_ptr());
                    return Err(io::Error::from(io::ErrorKind::OutOfMemory));
                };
                sys::lv_obj_add_flag(screen.as_ptr(), sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE);
                Ok((target, screen, reset))
            }
        })
        .map_err(io::Error::other)??;

        Ok(Self {
            _display: display,
            target,
            screen,
            reset,
            state,
        })
    }

    pub fn take(&self) -> bool {
        self.state.double_tap.replace(false)
    }

    pub fn position(&self, offset: i32, settled: bool) -> io::Result<()> {
        if !settled {
            self.state.first.set(None);
        }
        with_session(|_| {
            // SAFETY: The target is live and used under an exclusive LVGL session.
            unsafe {
                sys::lv_obj_set_style_translate_y(self.target.as_ptr(), offset, 0);
                if settled {
                    sys::lv_obj_add_flag(
                        self.target.as_ptr(),
                        sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE,
                    );
                } else {
                    sys::lv_obj_remove_flag(
                        self.target.as_ptr(),
                        sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE,
                    );
                }
            }
        })
        .map_err(io::Error::other)
    }
}

impl Drop for LogoTap<'_> {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The target and callback state remain live through deletion.
            unsafe {
                sys::lv_obj_remove_event_dsc(self.screen.as_ptr(), self.reset.as_ptr());
                sys::lv_obj_delete(self.target.as_ptr());
            }
        });
    }
}

unsafe extern "C" fn hit_test(event: *mut sys::lv_event_t) {
    // SAFETY: LVGL supplies live hit-test data and a live event target.
    unsafe {
        let target = sys::lv_event_get_current_target(event).cast::<sys::lv_obj_t>();
        let info = sys::lv_event_get_hit_test_info(event);
        let point = sys::lvgl_sys_hit_test_point(info);
        let mut bounds = sys::lv_area_t::default();
        sys::lv_obj_get_coords(target, &mut bounds);
        let x = i64::from(point.x) - i64::from(bounds.x1) - i64::from(RADIUS);
        let y = i64::from(point.y) - i64::from(bounds.y1) - i64::from(RADIUS);
        sys::lvgl_sys_hit_test_set_result(info, x * x + y * y <= i64::from(RADIUS * RADIUS));
    }
}

unsafe extern "C" fn reset_tap(event: *mut sys::lv_event_t) {
    // SAFETY: The screen callback is removed before its state is dropped.
    let state = unsafe { &*sys::lv_event_get_user_data(event).cast::<TapState>() };
    state.first.set(None);
}

unsafe extern "C" fn click(event: *mut sys::lv_event_t) {
    // SAFETY: LVGL supplies the registered state, kept alive by its owner.
    let state = unsafe { &*sys::lv_event_get_user_data(event).cast::<TapState>() };
    let now = Instant::now();
    if state
        .first
        .take()
        .is_some_and(|first| now.duration_since(first) <= DOUBLE_TAP)
    {
        state.double_tap.set(true);
    } else {
        state.first.set(Some(now));
    }
}
