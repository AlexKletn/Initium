use std::io;
use std::path::Path;

use initium_config::{BootloaderCoverConfig, BootloaderFrameConfig};

use super::{framebuffer, reserved};

pub struct BootloaderFrame {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl BootloaderFrame {
    pub fn capture(path: &Path, config: &BootloaderFrameConfig) -> io::Result<Self> {
        let mut frame = match &config.reserved_memory {
            Some(region) => reserved::capture(region)?,
            None => framebuffer::capture(path)?,
        };
        if let Some(cover) = &config.cover {
            frame.cover(cover)?;
        }
        Ok(frame)
    }

    pub(crate) fn new(width: u32, height: u32, pixels: Vec<u32>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(super) fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    fn cover(&mut self, cover: &BootloaderCoverConfig) -> io::Result<()> {
        let right = u64::from(cover.x) + u64::from(cover.width.get());
        let bottom = u64::from(cover.y) + u64::from(cover.height.get());
        let reference_width = u64::from(cover.reference_width.get());
        let reference_height = u64::from(cover.reference_height.get());
        if right > reference_width || bottom > reference_height {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cover exceeds reference frame",
            ));
        }

        let scale_x = |x| (x * u64::from(self.width) / reference_width) as usize;
        let scale_y = |y| (y * u64::from(self.height) / reference_height) as usize;
        let left = scale_x(u64::from(cover.x));
        let right = scale_x(right);
        let color = u32::from_be_bytes([0xff, cover.rgb[0], cover.rgb[1], cover.rgb[2]]);
        for y in scale_y(u64::from(cover.y))..scale_y(bottom) {
            let row = y * self.width as usize;
            self.pixels[row + left..row + right].fill(color);
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "frame.tests.rs"]
mod tests;
