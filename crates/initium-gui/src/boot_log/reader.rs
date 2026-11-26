use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;

use super::entry::Entry;

const MAX_ENTRIES: usize = 192;
const MAX_READS: usize = 8192;
const RECORD_BYTES: usize = 8192;

pub(super) fn read_kernel_log() -> io::Result<VecDeque<Entry>> {
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/kmsg")?;
    let mut entries = VecDeque::with_capacity(MAX_ENTRIES);
    let mut record = [0; RECORD_BYTES];

    for _ in 0..MAX_READS {
        match source.read(&mut record) {
            Ok(0) => break,
            Ok(size) => {
                if let Some(entry) = Entry::parse(&record[..size]) {
                    if entries.len() == MAX_ENTRIES {
                        entries.pop_front();
                    }
                    entries.push_back(entry);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error)
                if error.kind() == io::ErrorKind::Interrupted
                    || error.raw_os_error() == Some(libc::EPIPE) => {}
            Err(error) => return Err(error),
        }
    }

    Ok(entries)
}
