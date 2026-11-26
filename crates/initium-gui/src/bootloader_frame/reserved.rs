use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path};

use initium_config::BootloaderRegionConfig;

use super::{BootloaderFrame, layout::Layout};

const RESERVED_MEMORY: &str = "/sys/firmware/devicetree/base/reserved-memory";

pub(super) fn capture(config: &BootloaderRegionConfig) -> io::Result<BootloaderFrame> {
    let (offset, layout) = resolve(Path::new(RESERVED_MEMORY), config)?;
    let memory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_SYNC)
        .open("/dev/mem")?;
    layout.read_frame(&memory, offset)
}

fn resolve(root: &Path, config: &BootloaderRegionConfig) -> io::Result<(libc::off_t, Layout)> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid bootloader memory region",
        )
    };
    let mut components = Path::new(&config.node).components();
    let Some(Component::Normal(name)) = components.next() else {
        return Err(invalid());
    };
    if name != config.node.as_str() || components.next().is_some() {
        return Err(invalid());
    }

    let node = root.join(name);
    if !node.join("no-map").try_exists()? || !fs::read(root.join("ranges"))?.is_empty() {
        return Err(invalid());
    }

    let address_cells = read_cells(root, "#address-cells")?;
    let size_cells = read_cells(root, "#size-cells")?;
    let reg = fs::read(node.join("reg"))?;
    if reg.len() != (address_cells + size_cells) * 4 {
        return Err(invalid());
    }

    let (address, size) = reg.split_at(address_cells * 4);
    let address = read_value(address);
    let size = read_value(size);
    let width = config.width.get();
    let height = config.height.get();
    let stride = width.checked_mul(4).ok_or_else(invalid)?;
    let bytes = stride.checked_mul(height).ok_or_else(invalid)?;
    if address == 0 || address.checked_add(size).is_none() || u64::from(bytes) > size {
        return Err(invalid());
    }

    let offset = libc::off_t::try_from(address).map_err(|_| invalid())?;
    Ok((
        offset,
        Layout {
            width,
            height,
            stride,
            length: bytes,
            ..Layout::default()
        },
    ))
}

fn read_cells(root: &Path, property: &str) -> io::Result<usize> {
    let bytes = fs::read(root.join(property))?;
    let cells = match bytes.as_slice() {
        [0, 0, 0, 1] => 1,
        [0, 0, 0, 2] => 2,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Device Tree cell count",
            ));
        }
    };
    Ok(cells)
}

fn read_value(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0, |value, byte| (value << 8) | u64::from(*byte))
}

#[cfg(test)]
#[path = "reserved.tests.rs"]
mod tests;
