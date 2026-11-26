use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;

use initium_config::{HardwareItem, PowerMenuItem, Theme, UserspaceChoice};
use lvgl_rust::sys;

use crate::lvgl::with_session;
use crate::{
    BootLogPanel, BootloaderFrame, BootloaderImage, HardwarePanel, PowerMenu, UserspaceSelector,
};

use super::{drm_front_buffer::DrmFrontBuffer, drm_kms, drm_master::DrmMaster};

pub struct DisplayConnection {
    display: NonNull<sys::lv_display_t>,
    theme: Theme,
    drm_master: Option<DrmMaster>,
    front_buffer: RefCell<Option<Box<DrmFrontBuffer>>>,
}

impl DisplayConnection {
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub(crate) fn theme(&self) -> Theme {
        self.theme
    }

    pub fn show_userspace_selector(
        &self,
        choices: &[UserspaceChoice],
    ) -> io::Result<Option<UserspaceSelector<'_>>> {
        UserspaceSelector::create(self, choices)
    }

    pub fn show_power_menu<'display, 'config>(
        &'display self,
        items: &'config [PowerMenuItem],
    ) -> io::Result<Option<PowerMenu<'display, 'config>>> {
        PowerMenu::create(self, items)
    }

    pub fn show_hardware<'display, 'config>(
        &'display self,
        items: &'config [HardwareItem],
    ) -> io::Result<Option<HardwarePanel<'display, 'config>>> {
        HardwarePanel::create(self, items)
    }

    pub fn show_boot_log(&self) -> io::Result<BootLogPanel<'_>> {
        BootLogPanel::open(self)
    }

    pub fn show_bootloader_frame<'display, 'frame>(
        &'display self,
        frame: &'frame BootloaderFrame,
    ) -> io::Result<BootloaderImage<'display, 'frame>> {
        BootloaderImage::create(self, frame)
    }

    pub fn release_drm_master(&self) -> io::Result<()> {
        if let Some(master) = &self.drm_master {
            master.release()?;
        }
        Ok(())
    }

    pub fn drm_scanout(&self) -> io::Result<Option<u32>> {
        let master = self.drm_handle()?;
        drm_kms::scanout_framebuffer(master.fd(), master.connector_id)
    }

    pub fn wait_for_drm_vblank(&self) -> io::Result<()> {
        let master = self.drm_handle()?;
        drm_kms::wait_for_vblank(master.fd(), master.connector_id)
    }

    pub fn draw_into_drm_front_buffer(&self, framebuffer: u32) -> io::Result<()> {
        let mut attached = self.front_buffer.borrow_mut();
        if attached.is_some() {
            return Err(io::Error::other("DRM front buffer is already attached"));
        }
        let mut front = Box::new(DrmFrontBuffer::map(self.drm_handle()?.fd(), framebuffer)?);
        with_session(|_| {
            // SAFETY: The stored mapping stays live through this display's deletion.
            unsafe { front.attach(self.display) }
        })
        .map_err(io::Error::other)??;
        *attached = Some(front);
        Ok(())
    }

    pub fn publish_drm_front_buffer(&self, framebuffer: u32) -> io::Result<()> {
        let attached = self.front_buffer.borrow();
        let Some(front) = attached.as_ref().filter(|front| front.has_pending_pixels()) else {
            return Ok(());
        };
        let vblank = self.wait_for_drm_vblank();
        if self.drm_scanout()? == Some(framebuffer) {
            front.publish();
        }
        vblank
    }

    fn drm_handle(&self) -> io::Result<&DrmMaster> {
        self.drm_master
            .as_ref()
            .ok_or_else(|| io::Error::other("display is not connected through DRM"))
    }

    pub(super) fn set_drm_master(&mut self, master: DrmMaster) {
        self.drm_master = Some(master);
    }

    pub fn poll_events(&self) -> Result<(), crate::Error> {
        with_session(|lvgl| {
            lvgl.timer_handler();
        })
    }

    pub fn refresh(&self) -> Result<(), crate::Error> {
        with_session(|_| {
            // SAFETY: The display is live and the LVGL session is exclusive.
            unsafe { sys::lv_refr_now(self.display.as_ptr()) };
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut sys::lv_display_t {
        self.display.as_ptr()
    }

    pub(super) fn open(
        path: &Path,
        create: unsafe extern "C" fn() -> *mut sys::lv_display_t,
        configure: impl FnOnce(NonNull<sys::lv_display_t>, &CStr) -> sys::lv_result_t,
    ) -> io::Result<Self> {
        let file = CString::new(path.as_os_str().as_bytes())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        with_session(|_| {
            // SAFETY: The LVGL session provides exclusive access to the creator.
            let display = NonNull::new(unsafe { create() })
                .ok_or_else(|| io::Error::other("LVGL could not allocate a display"))?;

            let result = configure(display, &file);
            if result != sys::lv_result_t_LV_RESULT_OK {
                // SAFETY: This session exclusively owns the display, including after setup failure.
                unsafe { sys::lv_display_delete(display.as_ptr()) };
                return Err(io::Error::other(format!(
                    "LVGL could not configure {} (status {result})",
                    path.display()
                )));
            }

            Ok(Self {
                display,
                theme: Theme::default(),
                drm_master: None,
                front_buffer: RefCell::new(None),
            })
        })
        .map_err(io::Error::other)?
    }
}

impl Drop for DisplayConnection {
    fn drop(&mut self) {
        let _ = with_session(|_| {
            // SAFETY: The live display is owned by self and access is exclusive.
            unsafe { sys::lv_display_delete(self.display.as_ptr()) };
        });
    }
}

#[cfg(test)]
#[path = "connection.tests.rs"]
mod tests;
