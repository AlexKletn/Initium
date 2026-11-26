use std::fs::File;
use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::path::PathBuf;

use lvgl_rust::sys;

use crate::lvgl::with_session;

use super::{
    DisplayConnection, DisplayPort,
    drm_master::{DrmMaster, device_descriptors},
};

// LVGL references libdrm from a dependency archive scanned after this one.
#[cfg_attr(
    target_env = "musl",
    link(name = "drm", kind = "static", modifiers = "+whole-archive,-bundle")
)]
#[cfg_attr(not(target_env = "musl"), link(name = "drm"))]
unsafe extern "C" {
    fn initium_drm_select_connector(fd: libc::c_int, connector_id: *mut u32) -> libc::c_int;
    fn initium_drm_scanout(
        fd: libc::c_int,
        connector_id: u32,
        framebuffer: *mut u32,
    ) -> libc::c_int;
    fn initium_drm_wait_vblank(fd: libc::c_int, connector_id: u32) -> libc::c_int;
}

pub struct DrmKms {
    path: PathBuf,
    connector_id: Option<u32>,
}

impl DrmKms {
    pub fn new(path: impl Into<PathBuf>, connector_id: Option<u32>) -> Self {
        Self {
            path: path.into(),
            connector_id,
        }
    }

    pub fn resolve_connector(self) -> io::Result<Self> {
        let connector_id = self.connector_id()?;
        Ok(Self::new(self.path, Some(connector_id)))
    }

    pub fn scanout_framebuffer(&self) -> io::Result<Option<u32>> {
        let connector_id = self.connector_id()?;
        let card = File::open(&self.path)?;
        scanout_framebuffer(card.as_fd(), connector_id)
    }

    fn connector_id(&self) -> io::Result<u32> {
        if let Some(id) = self.connector_id {
            return Ok(id);
        }
        let card = File::open(&self.path)?;
        select_connector(card.as_fd()).map_err(|error| {
            io::Error::other(format!(
                "select a DRM connector on {}: {error}",
                self.path.display()
            ))
        })
    }
}

impl DisplayPort for DrmKms {
    type Connection = DisplayConnection;

    fn connect(&self) -> io::Result<Self::Connection> {
        with_session(|_| ()).map_err(io::Error::other)?;
        let connector_id = self.connector_id()?;
        let previous = device_descriptors(&self.path)?;
        let mut display =
            DisplayConnection::open(&self.path, sys::lv_linux_drm_create, |display, file| {
                // SAFETY: The session is exclusive, and file is a live C string.
                unsafe {
                    sys::lv_linux_drm_set_file(
                        display.as_ptr(),
                        file.as_ptr(),
                        i64::from(connector_id),
                    )
                }
            })?;
        display.set_drm_master(DrmMaster::capture(&self.path, &previous, connector_id)?);
        Ok(display)
    }
}

fn select_connector(fd: BorrowedFd<'_>) -> io::Result<u32> {
    let mut connector = 0;
    // SAFETY: The open card and writable output remain live throughout the call.
    let result = unsafe { initium_drm_select_connector(fd.as_raw_fd(), &mut connector) };
    if result < 0 {
        return Err(io::Error::from_raw_os_error(-result));
    }
    Ok(connector)
}

pub(super) fn wait_for_vblank(fd: BorrowedFd<'_>, connector_id: u32) -> io::Result<()> {
    // SAFETY: The descriptor remains live for the synchronous DRM request.
    let result = unsafe { initium_drm_wait_vblank(fd.as_raw_fd(), connector_id) };
    if result < 0 {
        return Err(io::Error::from_raw_os_error(-result));
    }
    Ok(())
}

pub(super) fn scanout_framebuffer(
    fd: BorrowedFd<'_>,
    connector_id: u32,
) -> io::Result<Option<u32>> {
    let mut framebuffer = 0;
    // SAFETY: The descriptor and writable output remain live for the call.
    let result = unsafe { initium_drm_scanout(fd.as_raw_fd(), connector_id, &mut framebuffer) };
    if result < 0 {
        return Err(io::Error::from_raw_os_error(-result));
    }
    Ok((framebuffer != 0).then_some(framebuffer))
}
