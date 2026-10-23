//! A full-screen report: the crash screen and the error screens.
//!
//! Allocation-free, because the panic handler uses it: nothing here touches
//! the heap, a [`Canvas`](super::canvas::Canvas) or `effie::system_table()`.
//! Text is drawn straight to the screen one glyph at a time, or printed on the
//! text console when there is no graphics output.

use effie::{
    protocols::{BltPixel, GraphicsOutput},
    tables::{BootServices, SystemTable},
};

use super::{
    chrome, console,
    font::Font,
    surface::{Direct, Surface},
    theme::{MARGIN, Palette},
};

/// Longest body line, in characters, so text stays readable on wide screens.
const MAX_COLUMNS: usize = 100;

pub struct Report<'a> {
    pub palette: &'a Palette,
    pub font: &'a Font,
    /// One line, centred, in the accent colour.
    pub title: &'a str,
    /// Wrapped to the screen. `\n` forces a line break.
    pub body: &'a str,
    /// Lines under the body, e.g. where the log was saved.
    pub notes: &'a [&'a str],
    /// Key hints for the footer, e.g. `("R", "reboot")`.
    pub keys: &'a [(&'a str, &'a str)],
}

/// Shows the report with graphics, falling back to the text console.
pub fn show(st: &SystemTable, boot_services: &BootServices, report: &Report) {
    if show_graphics(boot_services, report).is_none() {
        show_text(st, report);
    }
}

/// Splits `text` into lines of at most `columns` characters. Breaks at `\n`,
/// otherwise after the last space or `\` (so ESP paths break between
/// components) that fits, otherwise anywhere.
pub fn wrap(text: &str, columns: usize) -> Wrap<'_> {
    Wrap {
        rest: Some(text),
        columns: columns.max(1),
    }
}

pub struct Wrap<'a> {
    rest: Option<&'a str>,
    columns: usize,
}

impl<'a> Iterator for Wrap<'a> {
    type Item = &'a str;

    // Kept out of line: the screens walk the text twice, measuring then drawing.
    #[inline(never)]
    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest?;
        let bytes = rest.as_bytes();
        let mut chars = 0;
        let mut space = 0;
        let mut slash = 0;
        // Index of the first character that does not fit, or of a `\n`.
        let mut end = 0;
        while end < bytes.len() {
            let byte = bytes[end];
            if byte == b'\n' {
                self.rest = rest.get(end + 1..);
                return rest.get(..end);
            }
            // UTF-8 continuation bytes do not start a character.
            if byte & 0xC0 != 0x80 {
                if byte == b' ' {
                    space = end;
                }
                if chars == self.columns {
                    break;
                }
                chars += 1;
                if byte == b'\\' {
                    slash = end + 1;
                }
            }
            end += 1;
        }
        if end == bytes.len() {
            self.rest = None;
            return Some(rest);
        }
        // Prefer the last space (dropped), then just after the last `\` (ESP
        // paths break between components), else break anywhere. Every
        // offset is a character boundary.
        let (line, skip) = match (space, slash) {
            (0, 0) => (end, 0),
            (0, slash) => (slash, 0),
            (space, _) => (space, 1),
        };
        self.rest = rest.get(line + skip..).filter(|next| !next.is_empty());
        rest.get(..line)
    }
}

fn show_graphics(boot_services: &BootServices, report: &Report) -> Option<()> {
    let mut gop = boot_services.locate_protocol::<GraphicsOutput>().ok()?;
    let palette = report.palette;
    let font = report.font;
    let mut screen = Direct::new(&mut gop, font);
    let (width, height) = (screen.width(), screen.height());
    screen.fill(0, 0, width, height, palette.background);

    let top = chrome::header(&mut screen, palette);
    let (footer, y) = chrome::footer(&mut screen, palette);
    let hints_width: usize = report
        .keys
        .iter()
        .map(|(key, label)| key.chars().count() + 2 + label.chars().count())
        .sum::<usize>()
        + 4 * report.keys.len().saturating_sub(1);
    let mut x = width.saturating_sub(hints_width * font.width) / 2;
    for (i, (key, label)) in report.keys.iter().enumerate() {
        let gap = if i == 0 { "" } else { "    " };
        x = chrome::spans(
            &mut screen,
            palette,
            x,
            y,
            &[
                (gap, palette.dim),
                (key, palette.text),
                ("  ", palette.dim),
                (label, palette.dim),
            ],
        );
    }

    // The body is a left-aligned block, centred as a whole: centring each
    // wrapped line would look ragged. Measured first, then drawn.
    let columns = (width.saturating_sub(2 * MARGIN) / font.width).min(MAX_COLUMNS);
    let (lines, widest) = wrap(report.body, columns).fold((0, 0), |(lines, widest), line| {
        (lines + 1, widest.max(line.chars().count()))
    });
    let pitch = font.height + font.height / 3;
    let notes = match report.notes.len() {
        0 => 0,
        n => n + 1,
    };
    let content = (2 + lines + notes) * pitch;
    let mut y = top + footer.saturating_sub(top).saturating_sub(content) / 2;
    let mut line = |screen: &mut Direct, x: usize, text: &str, color: BltPixel| {
        if y + font.height <= footer {
            screen.text(x, y, text, color, palette.background);
        }
        y += pitch;
    };
    let centred = |text: &str| width.saturating_sub(font.text_width(text)) / 2;

    line(
        &mut screen,
        centred(report.title),
        report.title,
        palette.accent,
    );
    line(&mut screen, 0, "", palette.text);
    let left = width.saturating_sub(widest * font.width) / 2;
    for text in wrap(report.body, columns) {
        line(&mut screen, left, text, palette.text);
    }
    if !report.notes.is_empty() {
        line(&mut screen, 0, "", palette.text);
    }
    for note in report.notes {
        line(&mut screen, centred(note), note, palette.dim);
    }
    Some(())
}

fn show_text(st: &SystemTable, report: &Report) {
    let Ok(mut out) = st.con_out() else {
        return;
    };
    let palette = report.palette;
    let _ = out.set_attribute(palette.console);
    let _ = out.clear_screen();
    // A margin of two on each side.
    let columns = console::columns(&mut out).saturating_sub(4);

    console::print(&mut out, "\r\n");
    let _ = out.set_attribute(palette.console_bright);
    console::print(&mut out, "  ");
    console::print(&mut out, report.title);
    let _ = out.set_attribute(palette.console);
    console::print(&mut out, "\r\n\r\n");
    for line in wrap(report.body, columns) {
        console::print(&mut out, "  ");
        console::line(&mut out, line);
    }
    if !report.notes.is_empty() {
        console::print(&mut out, "\r\n");
    }
    for note in report.notes {
        console::print(&mut out, "  ");
        console::line(&mut out, note);
    }
    console::print(&mut out, "\r\n");
    for (key, label) in report.keys {
        for text in ["  ", key, " ", label, "  "] {
            console::print(&mut out, text);
        }
    }
    console::print(&mut out, "\r\n");
}
