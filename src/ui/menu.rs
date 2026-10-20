//! The boot menu: a list of entries in which folders expand in place.

use alloc::{format, vec::Vec};
use core::ptr;

use effie::protocols::{BltPixel, InputKey, SimpleTextInput};
use effie::{Protocol, WString};

use super::canvas::Canvas;
use crate::config::{Config, Entry, Kind};
use crate::error::{Context, Result};

/// Returns the entry to boot: the one the user picks, or the default entry
/// once the timeout runs out. Never returns a folder.
pub fn run(config: &Config, mut canvas: Option<Canvas>) -> Result<&Entry> {
    let mut con_in = effie::system_table()
        .con_in()
        .context("opening the console input")?;
    let mut menu = Menu::new(config);
    let mut countdown = config.timeout;

    loop {
        let rows = menu.rows();
        let entry = rows[menu.selected].entry;
        match &mut canvas {
            Some(canvas) => draw(canvas, &rows, menu.selected, &mut menu.scroll, countdown),
            None => print(&rows, menu.selected, countdown),
        }

        let key = match countdown {
            Some(0) if !is_folder(entry) => {
                log::info!("timeout, booting '{}'", entry.name);
                return Ok(entry);
            }
            Some(0) => {
                countdown = None;
                continue;
            }
            Some(seconds) => match key_within_a_second(&mut con_in) {
                Some(key) => {
                    countdown = None;
                    key
                }
                None => {
                    countdown = Some(seconds - 1);
                    continue;
                }
            },
            None => wait_for_key(&mut con_in)?,
        };

        match key.scan_code {
            InputKey::SCAN_UP => menu.selected = menu.selected.saturating_sub(1),
            InputKey::SCAN_DOWN => menu.selected = (menu.selected + 1).min(rows.len() - 1),
            InputKey::SCAN_NULL if key.unicode_char == u16::from(b'\r') => {
                if is_folder(entry) {
                    menu.toggle(entry);
                } else {
                    log::info!("selected '{}'", entry.name);
                    return Ok(entry);
                }
            }
            _ => {}
        }
    }
}

struct Menu<'a> {
    entries: &'a [Entry],
    /// Expanded folders, compared by address.
    open: Vec<&'a Entry>,
    /// Index into [`Menu::rows`].
    selected: usize,
    /// First row shown when the list is taller than the screen.
    scroll: usize,
}

/// One visible line of the menu.
struct Row<'a> {
    entry: &'a Entry,
    depth: usize,
    open: bool,
}

impl<'a> Menu<'a> {
    fn new(config: &'a Config) -> Self {
        let mut menu = Self {
            entries: &config.entries,
            open: Vec::new(),
            selected: 0,
            scroll: 0,
        };
        if let Some(default) = menu.open_default(config.default.as_deref()) {
            menu.selected = menu
                .rows()
                .iter()
                .position(|row| ptr::eq(row.entry, default))
                .unwrap_or(0);
        }
        menu
    }

    /// Finds the default entry and opens the folders leading to it. When the
    /// default is a folder with a default of its own, that folder is opened
    /// and its default followed, so the result is bootable whenever the
    /// config's defaults are.
    fn open_default(&mut self, id: Option<&str>) -> Option<&'a Entry> {
        let mut entry = find(self.entries, id?, &mut self.open)?;
        while let Kind::Folder(folder) = &entry.kind {
            let Some(inner) = folder
                .default
                .as_deref()
                .and_then(|id| find(&folder.entries, id, &mut self.open))
            else {
                break;
            };
            self.open.push(entry);
            entry = inner;
        }
        Some(entry)
    }

    fn is_open(&self, entry: &Entry) -> bool {
        self.open.iter().any(|&open| ptr::eq(open, entry))
    }

    fn toggle(&mut self, folder: &'a Entry) {
        match self.open.iter().position(|&open| ptr::eq(open, folder)) {
            Some(i) => {
                self.open.swap_remove(i);
            }
            None => self.open.push(folder),
        }
    }

    /// The entries currently shown, with open folders expanded.
    fn rows(&self) -> Vec<Row<'a>> {
        let mut rows = Vec::new();
        self.push_rows(self.entries, 0, &mut rows);
        rows
    }

    fn push_rows(&self, entries: &'a [Entry], depth: usize, rows: &mut Vec<Row<'a>>) {
        for entry in entries {
            let open = self.is_open(entry);
            rows.push(Row { entry, depth, open });
            if open && let Kind::Folder(folder) = &entry.kind {
                self.push_rows(&folder.entries, depth + 1, rows);
            }
        }
    }
}

/// Finds the entry with `id` anywhere in the tree, and pushes the folders
/// that contain it onto `path`.
fn find<'a>(entries: &'a [Entry], id: &str, path: &mut Vec<&'a Entry>) -> Option<&'a Entry> {
    for entry in entries {
        if entry.id == id {
            return Some(entry);
        }
        if let Kind::Folder(folder) = &entry.kind {
            path.push(entry);
            if let Some(found) = find(&folder.entries, id, path) {
                return Some(found);
            }
            path.pop();
        }
    }
    None
}

fn is_folder(entry: &Entry) -> bool {
    matches!(entry.kind, Kind::Folder(_))
}

fn wait_for_key(con_in: &mut Protocol<SimpleTextInput>) -> Result<InputKey> {
    let events = [con_in.wait_for_key_event()];
    loop {
        effie::system_table()
            .boot_services()
            .wait_for_event(&events)
            .context("waiting for a key")?;
        if let Ok(key) = con_in.read_key_stroke() {
            return Ok(key);
        }
    }
}

fn key_within_a_second(con_in: &mut Protocol<SimpleTextInput>) -> Option<InputKey> {
    let boot_services = effie::system_table().boot_services();
    for _ in 0..100 {
        if let Ok(key) = con_in.read_key_stroke() {
            return Some(key);
        }
        let _ = boot_services.stall(10_000);
    }
    None
}

// Graphical menu

const BACKGROUND: BltPixel = BltPixel::new(18, 12, 6);
const BAR: BltPixel = BltPixel::new(26, 17, 8);
const RULE: BltPixel = BltPixel::new(42, 28, 12);
const TEXT: BltPixel = BltPixel::new(205, 185, 150);
const DIM: BltPixel = BltPixel::new(105, 84, 58);
const ACCENT: BltPixel = BltPixel::new(230, 170, 55);

const MARGIN: usize = 20;
/// Vertical padding around the text in the header and footer bars.
const BAR_PADDING: usize = 12;

fn draw(
    canvas: &mut Canvas,
    rows: &[Row],
    selected: usize,
    scroll: &mut usize,
    countdown: Option<u32>,
) {
    canvas.clear(BACKGROUND);
    let top = draw_header(canvas);
    let bottom = draw_footer(canvas, countdown);
    draw_list(canvas, rows, selected, scroll, top, bottom);
    canvas.flush();
}

/// Draws text in several colours on a bar, returning the x just past it.
fn spans(canvas: &mut Canvas, mut x: usize, y: usize, spans: &[(&str, BltPixel)]) -> usize {
    for &(text, color) in spans {
        x = canvas.text(x, y, text, color, BAR);
    }
    x
}

/// Draws the title bar and returns the y just below it.
fn draw_header(canvas: &mut Canvas) -> usize {
    let height = canvas.font.height + 2 * BAR_PADDING;
    canvas.fill(0, 0, canvas.width, height, BAR);
    canvas.fill(0, height, canvas.width, 1, RULE);
    let version = concat!("  v", env!("CARGO_PKG_VERSION"));
    spans(
        canvas,
        MARGIN,
        BAR_PADDING,
        &[("bread", ACCENT), (version, DIM)],
    );
    height + 1
}

/// Draws the key help bar and returns the y of its top edge.
fn draw_footer(canvas: &mut Canvas, countdown: Option<u32>) -> usize {
    let height = canvas.font.height + 2 * BAR_PADDING;
    let top = canvas.height.saturating_sub(height + 1);
    let y = top + 1 + BAR_PADDING;
    canvas.fill(0, top, canvas.width, 1, RULE);
    canvas.fill(0, top + 1, canvas.width, height, BAR);

    spans(
        canvas,
        MARGIN,
        y,
        &[
            ("UP", TEXT),
            (" / ", DIM),
            ("DOWN", TEXT),
            ("  navigate", DIM),
        ],
    );

    if let Some(seconds) = countdown {
        let message = format!("Booting in {seconds}s");
        let x = canvas
            .width
            .saturating_sub(canvas.font.text_width(&message))
            / 2;
        spans(canvas, x, y, &[(&message, ACCENT)]);
    }

    let hint = [("ENTER", TEXT), ("  boot", DIM)];
    let hint_width = canvas.font.text_width("ENTER  boot");
    spans(
        canvas,
        canvas.width.saturating_sub(MARGIN + hint_width),
        y,
        &hint,
    );

    top
}

/// Draws the entries between `top` and `bottom`, scrolling to keep the
/// selection visible.
fn draw_list(
    canvas: &mut Canvas,
    rows: &[Row],
    selected: usize,
    scroll: &mut usize,
    top: usize,
    bottom: usize,
) {
    const ROW_PADDING: usize = 7;
    const GAP: usize = 10;

    let (glyph_width, glyph_height) = (canvas.font.width, canvas.font.height);
    let pitch = glyph_height + 2 * ROW_PADDING + GAP;
    let list_width = (canvas.width * 3 / 5).clamp(480, 900);
    let left = canvas.width.saturating_sub(list_width) / 2;
    let available = bottom.saturating_sub(top);
    let capacity = ((available + GAP) / pitch).max(1);

    *scroll = (*scroll)
        .clamp((selected + 1).saturating_sub(capacity), selected)
        .min(rows.len().saturating_sub(capacity));
    let visible = &rows[*scroll..(*scroll + capacity).min(rows.len())];
    let list_top = top + available.saturating_sub(visible.len() * pitch - GAP) / 2;
    let arrow_x = left + list_width / 2 - glyph_width / 2;

    if *scroll > 0 {
        let y = list_top.saturating_sub(glyph_height + 4);
        canvas.text(arrow_x, y, "^", DIM, BACKGROUND);
    }

    for (i, row) in visible.iter().enumerate() {
        let x = left + 10 + 2 * row.depth * glyph_width;
        let y = list_top + i * pitch + ROW_PADDING;
        let marker = match (&row.entry.kind, row.open) {
            (Kind::Folder(_), false) => "+ ",
            (Kind::Folder(_), true) => "- ",
            _ => "  ",
        };

        let (fg, bg) = if *scroll + i == selected {
            let width = canvas.font.text_width(marker) + canvas.font.text_width(&row.entry.name);
            canvas.fill(
                x.saturating_sub(8),
                y.saturating_sub(3),
                width + 16,
                glyph_height + 6,
                ACCENT,
            );
            (BACKGROUND, ACCENT)
        } else if is_folder(row.entry) {
            (ACCENT, BACKGROUND)
        } else {
            (TEXT, BACKGROUND)
        };
        let x = canvas.text(x, y, marker, fg, bg);
        canvas.text(x, y, &row.entry.name, fg, bg);
    }

    if *scroll + visible.len() < rows.len() {
        let y = list_top + visible.len() * pitch;
        if y + glyph_height <= bottom {
            canvas.text(arrow_x, y, "v", DIM, BACKGROUND);
        }
    }
}

// Text console fallback

const NORMAL: usize = 0x07;
const BRIGHT: usize = 0x0F;
const INVERTED: usize = 0x70;

fn print(rows: &[Row], selected: usize, countdown: Option<u32>) {
    let Ok(mut out) = effie::system_table().con_out() else {
        return;
    };
    let _ = out.enable_cursor(false);
    let _ = out.clear_screen();
    let mut line = |attribute: usize, text: &str| {
        let _ = out.set_attribute(attribute);
        let _ = out.output_line(&WString::from(text));
    };

    line(BRIGHT, concat!("  bread  v", env!("CARGO_PKG_VERSION")));
    line(NORMAL, "");
    for (i, row) in rows.iter().enumerate() {
        let marker = match (&row.entry.kind, row.open) {
            (Kind::Folder(_), false) => "  [+]",
            (Kind::Folder(_), true) => "  [-]",
            _ => "",
        };
        let indent = 2 * (row.depth + 1);
        let text = format!("{:indent$}{}{marker}", "", row.entry.name);
        line(if i == selected { INVERTED } else { NORMAL }, &text);
    }
    line(NORMAL, "");
    line(NORMAL, "  UP / DOWN  navigate    ENTER  boot / open");
    if let Some(seconds) = countdown {
        line(NORMAL, &format!("  Booting in {seconds}s"));
    }
}
