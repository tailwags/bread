//! The panic handler. It reports the panic on every channel that still
//! works, then waits for a key and reboots.
//!
//! The panic is logged like any other error, so it reaches the serial port
//! and ends the log history, which is saved to `\EFI\bread\bread.log`. It is
//! also shown on the screen (graphics, or the text console as a fallback).
//! Every step is best-effort and allocation-free, since the heap may be what
//! failed. A panic inside the handler halts.

use core::fmt::Write;
use core::panic::PanicInfo;

use effie::protocols::{BltPixel, FileHandle, FileMode, GraphicsOutput, SimpleTextOutput};
use effie::tables::{BootServices, ResetType, SystemTable};
use effie::{Status, WStr, w};
use effie_log::Buf;

use crate::ui::font::EMBEDDED as FONT;

const LOG_PATH: &WStr = w!("\\EFI\\bread\\bread.log");
const LOG_SAVED: &str = "Log saved to \\EFI\\bread\\bread.log";
const LOG_HEADER: &[u8] = b"bread panic log\r\n\
    ===============\r\n\
    bread wrote this file when it hit an unrecoverable error. Please attach it\r\n\
    to a bug report at https://github.com/tailwags/bread\r\n\r\n";

const BACKGROUND: BltPixel = BltPixel::new(40, 0, 0);
const FOREGROUND: BltPixel = BltPixel::new(255, 255, 255);

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    if !effie_log::report_panic(info) {
        halt();
    }

    let mut location = Buf::<160>::new();
    let _ = match info.location() {
        Some(at) => write!(
            location,
            "PANIC at {}:{}:{}",
            at.file(),
            at.line(),
            at.column()
        ),
        None => write!(location, "PANIC"),
    };
    let mut message = Buf::<256>::new();
    let _ = write!(message, "{}", info.message());
    let (location, message) = (location.as_str(), message.as_str());

    // The screen, the file system and the keyboard all need boot services.
    // SAFETY: effie stores a valid system table pointer before calling `main`.
    let system_table = effie::system_table_raw().map(|table| unsafe { table.as_ref() });
    if let Some(st) = system_table
        && let Some(boot_services) = st.boot_services_opt()
    {
        let saved = save_log().is_some();
        let lines = [
            location,
            message,
            if saved {
                LOG_SAVED
            } else {
                "The log could not be saved"
            },
            "Press any key to reboot",
        ];
        if show_graphics(boot_services, &lines).is_none() {
            show_text(st, &lines);
        }
        if wait_for_key(st, boot_services).is_some() {
            st.runtime_services().reset_system(ResetType::Cold);
        }
    }
    halt()
}

fn halt() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Writes the recent log, which ends with the panic, to
/// `\EFI\bread\bread.log`, replacing any previous log.
fn save_log() -> Option<()> {
    // Taken first, so the log ends with the panic.
    let (older, newer) = effie_log::take_history();
    let mut root = crate::esp::root().ok()?;
    // Opening with CREATE would keep the old file's contents past our end.
    if let Ok(old) = root.open(LOG_PATH, FileMode::READ | FileMode::WRITE) {
        let _ = old.delete();
    }
    let mode = FileMode::READ | FileMode::WRITE | FileMode::CREATE;
    let mut file = root.open(LOG_PATH, mode).ok()?;

    for part in [LOG_HEADER, b"---- log, oldest first ----\r\n", older, newer] {
        write_all(&mut file, part)?;
    }
    file.flush().ok()
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

/// Shows `lines` centred on a red screen, with a blank line between each.
fn show_graphics(boot_services: &BootServices, lines: &[&str]) -> Option<()> {
    let mut gop = boot_services.locate_protocol::<GraphicsOutput>().ok()?;
    let info = gop.current_mode_info();
    let width = info.horizontal_resolution as usize;
    let height = info.vertical_resolution as usize;
    gop.blt_video_fill(BACKGROUND, 0, 0, width, height).ok()?;

    let top = height.saturating_sub((2 * lines.len() - 1) * FONT.height) / 2;
    for (i, line) in lines.iter().enumerate() {
        let x = width.saturating_sub(FONT.text_width(line)) / 2;
        draw_line(&mut gop, x, top + 2 * i * FONT.height, line);
    }
    Some(())
}

/// Draws text straight to the screen one character at a time, so no
/// full-screen buffer is needed.
fn draw_line(gop: &mut GraphicsOutput, mut x: usize, y: usize, text: &str) {
    let mut cell = [BACKGROUND; FONT.width * FONT.height];
    for c in text.chars() {
        let glyph = FONT.glyph(c);
        for (i, pixel) in cell.iter_mut().enumerate() {
            let set = glyph.is_set(i % FONT.width, i / FONT.width);
            *pixel = if set { FOREGROUND } else { BACKGROUND };
        }
        let _ = gop.blt_buffer_to_video(&cell, 0, 0, x, y, FONT.width, FONT.height, 0);
        x += FONT.width;
    }
}

/// Prints `lines` on the text console.
fn show_text(st: &SystemTable, lines: &[&str]) {
    let Ok(mut out) = st.con_out() else {
        return;
    };
    for line in lines {
        print_line(&mut out, line);
    }
}

/// Converts to UTF-16 in fixed-size pieces, since there is no heap to use.
fn print_line(out: &mut SimpleTextOutput, text: &str) {
    let crlf = [u16::from(b'\r'), u16::from(b'\n')];
    let mut units = text
        .encode_utf16()
        .chain(crlf)
        .filter(|&unit| unit != 0)
        .peekable();
    while units.peek().is_some() {
        let mut buf = [0u16; 64];
        let len = buf[..63]
            .iter_mut()
            .zip(&mut units)
            .map(|(slot, unit)| *slot = unit)
            .count();
        if let Some(piece) = WStr::from_slice(&buf[..=len]) {
            let _ = out.output_string(piece);
        }
    }
}

/// Returns once a key is pressed, or `None` if there is no keyboard.
fn wait_for_key(st: &SystemTable, boot_services: &BootServices) -> Option<()> {
    let mut con_in = st.con_in().ok()?;
    loop {
        match con_in.read_key_stroke() {
            Ok(_) => return Some(()),
            Err(Status::NOT_READY) => {
                let _ = boot_services.stall(10_000);
            }
            Err(_) => return None,
        }
    }
}
