//! A full-screen report: the crash screen and the error screens.
//!
//! Allocation-free, because the panic handler uses it: nothing here touches
//! the heap, a [`Canvas`](super::canvas::Canvas) or `effie::system_table()`.
//! Text is drawn straight to the screen one glyph at a time, or printed on the
//! text console when there is no graphics output.

use effie::protocols::{BltPixel, GraphicsOutput, InputKey, SimpleTextOutput};
use effie::tables::{BootServices, SystemTable};
use effie::{Status, WStr};

use super::font::Font;
use super::{BAR_PADDING, MARGIN};

/// Longest body line, in characters, so text stays readable on wide screens.
const MAX_COLUMNS: usize = 100;
/// Pixels in the buffer a glyph is drawn into before it goes to the screen.
const CELL: usize = 2048;

pub enum Style {
    /// bread itself failed.
    Crash,
    /// An expected failure, in the menu's colours.
    Error,
}

struct Palette {
    background: BltPixel,
    bar: BltPixel,
    rule: BltPixel,
    text: BltPixel,
    dim: BltPixel,
    accent: BltPixel,
    /// Text console attributes for the body and the title.
    console: (usize, usize),
}

impl Style {
    fn palette(&self) -> Palette {
        match self {
            Style::Crash => Palette {
                background: BltPixel::new(40, 0, 0),
                bar: BltPixel::new(56, 6, 6),
                rule: BltPixel::new(84, 16, 16),
                text: BltPixel::new(255, 255, 255),
                dim: BltPixel::new(205, 150, 150),
                accent: BltPixel::new(255, 180, 160),
                // White on red.
                console: (0x4F, 0x4F),
            },
            Style::Error => Palette {
                background: super::BACKGROUND,
                bar: super::BAR,
                rule: super::RULE,
                text: super::TEXT,
                dim: super::DIM,
                accent: super::ACCENT,
                console: (0x07, 0x0F),
            },
        }
    }
}

pub struct Report<'a> {
    pub style: Style,
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
        let (mut chars, mut space, mut slash) = (0, 0, 0);
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
    let info = gop.current_mode_info();
    let width = info.horizontal_resolution as usize;
    let height = info.vertical_resolution as usize;
    let palette = report.style.palette();
    let font = report.font;
    gop.blt_video_fill(palette.background, 0, 0, width, height)
        .ok()?;
    let mut screen = Screen {
        gop: &mut gop,
        font,
    };

    // Header and footer bars, like the menu's.
    let bar = font.height + 2 * BAR_PADDING;
    let footer = height.saturating_sub(bar + 1);
    screen.fill(0, 0, width, bar, palette.bar);
    screen.fill(0, bar, width, 1, palette.rule);
    screen.fill(0, footer, width, 1, palette.rule);
    screen.fill(0, footer + 1, width, bar, palette.bar);
    let x = screen.text(MARGIN, BAR_PADDING, "bread", palette.accent, palette.bar);
    let version = concat!("  v", env!("CARGO_PKG_VERSION"));
    screen.text(x, BAR_PADDING, version, palette.dim, palette.bar);

    let hints_width: usize = report
        .keys
        .iter()
        .map(|(key, label)| key.chars().count() + 2 + label.chars().count())
        .sum::<usize>()
        + 4 * report.keys.len().saturating_sub(1);
    let mut x = width.saturating_sub(hints_width * font.width) / 2;
    for (i, (key, label)) in report.keys.iter().enumerate() {
        let gap = if i == 0 { "" } else { "    " };
        x = screen.text(x, footer + 1 + BAR_PADDING, gap, palette.dim, palette.bar);
        x = screen.text(x, footer + 1 + BAR_PADDING, key, palette.text, palette.bar);
        x = screen.text(x, footer + 1 + BAR_PADDING, "  ", palette.dim, palette.bar);
        x = screen.text(x, footer + 1 + BAR_PADDING, label, palette.dim, palette.bar);
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
    let top = bar + 1;
    let mut y = top + footer.saturating_sub(top).saturating_sub(content) / 2;
    let mut line = |screen: &mut Screen, x: usize, text: &str, color: BltPixel| {
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

/// Draws on the screen directly, without a full-screen buffer.
struct Screen<'a> {
    gop: &'a mut GraphicsOutput,
    font: &'a Font,
}

impl Screen<'_> {
    fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, color: BltPixel) {
        let _ = self.gop.blt_video_fill(color, x, y, width, height);
    }

    /// Draws `text` one glyph at a time and returns the x just past it.
    /// Glyphs too big for [`CELL`] are drawn in horizontal strips.
    fn text(&mut self, mut x: usize, y: usize, text: &str, fg: BltPixel, bg: BltPixel) -> usize {
        let font = self.font;
        let width = font.width.min(CELL);
        let strip = (CELL / width).min(font.height);
        let mut cell = [bg; CELL];
        for c in text.chars() {
            let glyph = font.glyph(c);
            // Not `step_by`, which costs a panic path.
            let mut top = 0;
            while top < font.height {
                let rows = strip.min(font.height - top);
                let pixels = &mut cell[..width * rows];
                for (i, pixel) in pixels.iter_mut().enumerate() {
                    let set = glyph.is_set(i % width, top + i / width);
                    *pixel = if set { fg } else { bg };
                }
                let _ = self
                    .gop
                    .blt_buffer_to_video(pixels, 0, 0, x, y + top, width, rows, 0);
                top += strip;
            }
            x += font.width;
        }
        x
    }
}

fn show_text(st: &SystemTable, report: &Report) {
    let Ok(mut out) = st.con_out() else {
        return;
    };
    let (normal, bright) = report.style.palette().console;
    let _ = out.set_attribute(normal);
    let _ = out.clear_screen();
    let mode = out.current_mode().mode;
    let columns = usize::try_from(mode)
        .ok()
        .and_then(|mode| out.query_mode(mode).ok())
        .map_or(80, |(columns, _)| columns);
    // A margin of two on each side.
    let columns = columns.saturating_sub(4);

    print(&mut out, "\r\n");
    let _ = out.set_attribute(bright);
    print(&mut out, "  ");
    print(&mut out, report.title);
    let _ = out.set_attribute(normal);
    print(&mut out, "\r\n\r\n");
    for line in wrap(report.body, columns) {
        print(&mut out, "  ");
        print(&mut out, line);
        print(&mut out, "\r\n");
    }
    if !report.notes.is_empty() {
        print(&mut out, "\r\n");
    }
    for note in report.notes {
        print(&mut out, "  ");
        print(&mut out, note);
        print(&mut out, "\r\n");
    }
    print(&mut out, "\r\n");
    for (key, label) in report.keys {
        for text in ["  ", key, " ", label, "  "] {
            print(&mut out, text);
        }
    }
    print(&mut out, "\r\n");
}

/// Converts to UTF-16 in fixed-size pieces, since there is no heap to use.
fn print(out: &mut SimpleTextOutput, text: &str) {
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

/// How long the keyboard must be quiet before a key counts, so a key still
/// held from the menu (auto-repeating) cannot dismiss a screen before it is
/// seen. Longer than the usual 500 ms typematic delay.
const QUIET_MS: usize = 600;
const POLL_MS: usize = 10;

/// Discards pending keys, then waits for a key that `accept` takes. Returns
/// `None` once `timeout` seconds have passed, or straight away if there is no
/// keyboard and no timeout.
pub fn wait_for_key(
    st: &SystemTable,
    boot_services: &BootServices,
    timeout: Option<u32>,
    accept: impl Fn(&InputKey) -> bool,
) -> Option<InputKey> {
    let mut con_in = st.con_in().ok();
    if let Some(con_in) = &mut con_in {
        let _ = con_in.reset(false);
    }
    let mut left = timeout.map(|seconds| seconds as usize * 1000);
    let mut quiet = 0;
    loop {
        if let Some(left) = &mut left {
            *left = left.checked_sub(POLL_MS)?;
        }
        match con_in.as_mut().map(|con_in| con_in.read_key_stroke()) {
            Some(Ok(key)) if quiet >= QUIET_MS => {
                if accept(&key) {
                    return Some(key);
                }
            }
            Some(Ok(_)) => quiet = 0,
            Some(Err(Status::NOT_READY)) => quiet += POLL_MS,
            // No keyboard: wait out the timeout, if there is one.
            _ if timeout.is_some() => con_in = None,
            _ => return None,
        }
        let _ = boot_services.stall(POLL_MS * 1000);
    }
}
