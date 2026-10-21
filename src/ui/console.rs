//! The text console, for when there is no graphics output.

use effie::WStr;
use effie::protocols::SimpleTextOutput;

/// Prints `text`, converting to UTF-16 in fixed-size pieces so it needs no
/// heap.
pub fn print(out: &mut SimpleTextOutput, text: &str) {
    let mut units = text.encode_utf16().filter(|&unit| unit != 0).peekable();
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

/// Prints `text` and a line break.
pub fn line(out: &mut SimpleTextOutput, text: &str) {
    print(out, text);
    print(out, "\r\n");
}

/// The width of the current text mode, in characters.
pub fn columns(out: &mut SimpleTextOutput) -> usize {
    let mode = out.current_mode().mode;
    usize::try_from(mode)
        .ok()
        .and_then(|mode| out.query_mode(mode).ok())
        .map_or(80, |(columns, _)| columns)
}
