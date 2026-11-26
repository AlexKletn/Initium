use std::cell::Cell;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

unsafe extern "C" {
    fn drmDropMaster(fd: libc::c_int) -> libc::c_int;
}

pub(super) struct DrmMaster {
    fd: OwnedFd,
    held: Cell<bool>,
    pub connector_id: u32,
}

impl DrmMaster {
    pub fn capture(path: &Path, previous: &HashSet<RawFd>, connector_id: u32) -> io::Result<Self> {
        let active = device_descriptors(path)?;
        let mut added = active.difference(previous);
        let Some(&fd) = added.next() else {
            return Err(io::Error::other("LVGL opened no DRM descriptor"));
        };
        if added.next().is_some() {
            return Err(io::Error::other("LVGL opened multiple DRM descriptors"));
        }

        // SAFETY: fcntl duplicates an existing descriptor and returns a new owned descriptor.
        let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: The duplicate is valid and belongs to this object.
        let fd = unsafe { OwnedFd::from_raw_fd(duplicate) };
        Ok(Self {
            fd,
            held: Cell::new(true),
            connector_id,
        })
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }

    pub fn release(&self) -> io::Result<()> {
        if !self.held.get() {
            return Ok(());
        }
        // SAFETY: This is a live duplicate of the descriptor that owns DRM master.
        let result = unsafe { drmDropMaster(self.fd.as_raw_fd()) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        self.held.set(false);
        Ok(())
    }
}

pub(super) fn device_descriptors(path: &Path) -> io::Result<HashSet<RawFd>> {
    let device = fs::metadata(path)?;
    let mut descriptors = HashSet::new();
    for entry in fs::read_dir("/proc/self/fd")? {
        let entry = entry?;
        let Some(fd) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        if fs::metadata(entry.path())
            .is_ok_and(|file| file.file_type().is_char_device() && file.rdev() == device.rdev())
        {
            descriptors.insert(fd);
        }
    }
    Ok(descriptors)
}

#[cfg(test)]
#[path = "drm_master.tests.rs"]
mod tests;
