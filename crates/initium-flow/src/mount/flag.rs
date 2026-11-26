use libc::c_ulong;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum MountFlag {
    ReadOnly,
    NoSuid,
    NoDev,
    NoExec,
}

impl MountFlag {
    pub fn bits(&self) -> c_ulong {
        match self {
            Self::ReadOnly => libc::MS_RDONLY,
            Self::NoSuid => libc::MS_NOSUID,
            Self::NoDev => libc::MS_NODEV,
            Self::NoExec => libc::MS_NOEXEC,
        }
    }
}
