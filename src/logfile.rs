//! `\EFI\bread\bread.log`: the recent log, saved when something goes wrong.
//!
//! Allocation-free, because the panic handler uses it.

use effie::protocols::{FileHandle, FileMode};
use effie::{WStr, w};

const PATH: &WStr = w!("\\EFI\\bread\\bread.log");
const HEADER: &[u8] = b"bread log\r\n\
    =========\r\n\
    bread wrote this file after an error. If it looks like a bug in bread,\r\n\
    please attach it to a report at https://github.com/tailwags/bread\r\n\r\n";

/// Writes the log, given as two consecutive parts, replacing any previous
/// log. Needs boot services.
pub fn save(older: &[u8], newer: &[u8]) -> Option<()> {
    let mut root = crate::esp::root().ok()?;
    // Opening with CREATE would keep the old file's contents past our end.
    if let Ok(old) = root.open(PATH, FileMode::READ | FileMode::WRITE) {
        let _ = old.delete();
    }
    let mode = FileMode::READ | FileMode::WRITE | FileMode::CREATE;
    let mut file = root.open(PATH, mode).ok()?;

    for part in [HEADER, b"---- log, oldest first ----\r\n", older, newer] {
        write_all(&mut file, part)?;
    }
    file.flush().ok()
}

/// A line for the screen saying whether [`save`] worked.
pub fn note(saved: bool) -> &'static str {
    if saved {
        "Log saved to \\EFI\\bread\\bread.log"
    } else {
        "The log could not be saved"
    }
}

fn write_all(file: &mut FileHandle, mut data: &[u8]) -> Option<()> {
    while !data.is_empty() {
        match file.write(data) {
            Ok(0) | Err(_) => return None,
            Ok(n) => data = &data[n..],
        }
    }
    Some(())
}
