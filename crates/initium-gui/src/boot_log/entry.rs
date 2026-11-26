use std::ffi::CString;

use super::level::Level;

pub(crate) struct Entry {
    pub(super) timestamp: CString,
    pub(super) message: CString,
    pub(super) level: Level,
}

impl Entry {
    pub fn parse(record: &[u8]) -> Option<Self> {
        let line = record.split(|byte| *byte == b'\n').next()?;
        let separator = line.iter().position(|byte| *byte == b';')?;
        let mut header = std::str::from_utf8(&line[..separator]).ok()?.split(',');
        let priority = header.next()?.parse::<u8>().ok()?;
        header.next()?.parse::<u64>().ok()?;
        let micros = header.next()?.parse::<u64>().ok()?;
        let decoded = decode_message(&line[separator + 1..]);
        let message = String::from_utf8_lossy(&decoded);
        let message = message.trim();
        if message.is_empty() {
            return None;
        }

        Some(Self {
            timestamp: CString::new(format!(
                "{}.{:03}",
                micros / 1_000_000,
                micros / 1_000 % 1_000
            ))
            .ok()?,
            message: CString::new(message).ok()?,
            level: Level::from_priority(priority),
        })
    }
}

fn decode_message(mut input: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(input.len());
    while let Some((&first, rest)) = input.split_first() {
        let (value, remaining) = match input {
            [b'\\', b'x', high, low, remaining @ ..] => {
                let hex = [*high, *low];
                match std::str::from_utf8(&hex)
                    .ok()
                    .and_then(|digits| u8::from_str_radix(digits, 16).ok())
                {
                    Some(value) => (value, remaining),
                    None => (first, rest),
                }
            }
            _ => (first, rest),
        };
        decoded.push(if value.is_ascii_control() {
            b' '
        } else {
            value
        });
        input = remaining;
    }
    decoded
}

#[cfg(test)]
#[path = "entry.tests.rs"]
mod tests;
