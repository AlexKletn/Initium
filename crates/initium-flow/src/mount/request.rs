use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::ptr;

use libc::c_ulong;

use super::config::MountConfig;

pub(super) struct MountRequest {
    source: Option<CString>,
    target: CString,
    filesystem: CString,
    flags: c_ulong,
    options: Option<CString>,
}

impl TryFrom<MountConfig> for MountRequest {
    type Error = io::Error;

    fn try_from(config: MountConfig) -> io::Result<Self> {
        if !config.target.is_absolute() || config.filesystem.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "mount needs an absolute target path and a nonempty filesystem type",
            ));
        }

        let invalid_string = |error| io::Error::new(io::ErrorKind::InvalidInput, error);
        let source = config
            .source
            .map(CString::new)
            .transpose()
            .map_err(invalid_string)?;
        let target = CString::new(config.target.as_os_str().as_bytes()).map_err(invalid_string)?;
        let filesystem = CString::new(config.filesystem).map_err(invalid_string)?;
        let options = config
            .options
            .map(CString::new)
            .transpose()
            .map_err(invalid_string)?;
        let flags = config.flags.iter().fold(0, |bits, flag| bits | flag.bits());

        Ok(Self {
            source,
            target,
            filesystem,
            flags,
            options,
        })
    }
}

impl MountRequest {
    pub fn execute(self) -> io::Result<()> {
        let source = self
            .source
            .as_ref()
            .map_or(ptr::null(), |source| source.as_ptr());
        let options = self
            .options
            .as_ref()
            .map_or(ptr::null(), |options| options.as_ptr());

        // SAFETY: Every non-null argument points to a live, NUL-terminated buffer
        // for the duration of the call.
        let result = unsafe {
            libc::mount(
                source,
                self.target.as_ptr(),
                self.filesystem.as_ptr(),
                self.flags,
                options.cast(),
            )
        };
        if result == 0 {
            return Ok(());
        }
        Err(io::Error::last_os_error())
    }
}
