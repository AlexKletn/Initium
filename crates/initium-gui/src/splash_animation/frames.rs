use std::io::{self, BufRead};

pub(super) fn read_frame(source: &mut impl BufRead, pixels: &mut [u32]) -> io::Result<bool> {
    if source.fill_buf()?.is_empty() {
        return Ok(false);
    }

    let mut remaining = pixels;
    while !remaining.is_empty() {
        let mut run = [0; 8];
        source.read_exact(&mut run)?;
        let count = u32::from_le_bytes([run[0], run[1], run[2], run[3]]) as usize;
        if count == 0 || count > remaining.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid splash frame run",
            ));
        }
        let (segment, rest) = remaining.split_at_mut(count);
        segment.fill(u32::from_le_bytes([run[4], run[5], run[6], run[7]]));
        remaining = rest;
    }

    Ok(true)
}

#[cfg(test)]
#[path = "frames.tests.rs"]
mod tests;
