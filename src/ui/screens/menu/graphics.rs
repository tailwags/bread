//! The menu on a [`Canvas`].

use alloc::format;

use super::tree::{Row, is_folder};
use crate::ui::canvas::Canvas;
use crate::ui::chrome;
use crate::ui::theme::{MARGIN, MENU};

pub fn draw(
    canvas: &mut Canvas,
    rows: &[Row],
    selected: usize,
    scroll: &mut usize,
    countdown: Option<u32>,
    firmware_setup: bool,
) {
    canvas.clear(MENU.background);
    let top = chrome::header(canvas, &MENU);
    let bottom = draw_footer(canvas, countdown, firmware_setup);
    draw_list(canvas, rows, selected, scroll, top, bottom);
    canvas.flush();
}

/// Draws the key help bar and returns the y of its top edge.
fn draw_footer(canvas: &mut Canvas, countdown: Option<u32>, firmware_setup: bool) -> usize {
    let (top, y) = chrome::footer(canvas, &MENU);
    let x = chrome::spans(
        canvas,
        &MENU,
        MARGIN,
        y,
        &[
            ("UP", MENU.text),
            (" / ", MENU.dim),
            ("DOWN", MENU.text),
            ("  navigate", MENU.dim),
        ],
    );
    if firmware_setup {
        chrome::spans(
            canvas,
            &MENU,
            x,
            y,
            &[("    F2", MENU.text), ("  firmware setup", MENU.dim)],
        );
    }

    if let Some(seconds) = countdown {
        let message = format!("Booting in {seconds}s");
        let x = canvas
            .width
            .saturating_sub(canvas.font.text_width(&message))
            / 2;
        chrome::spans(canvas, &MENU, x, y, &[(&message, MENU.accent)]);
    }

    let hint = [("ENTER", MENU.text), ("  boot", MENU.dim)];
    let hint_width = canvas.font.text_width("ENTER  boot");
    chrome::spans(
        canvas,
        &MENU,
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
        canvas.text(arrow_x, y, "^", MENU.dim, MENU.background);
    }

    for (i, row) in visible.iter().enumerate() {
        let x = left + 10 + 2 * row.depth * glyph_width;
        let y = list_top + i * pitch + ROW_PADDING;
        let marker = match (is_folder(row.entry), row.open) {
            (true, false) => "+ ",
            (true, true) => "- ",
            _ => "  ",
        };
        let name = &row.entry.name;

        let (fg, bg) = if *scroll + i == selected {
            let width = canvas.font.text_width(marker) + canvas.font.text_width(name);
            canvas.fill(
                x.saturating_sub(8),
                y.saturating_sub(3),
                width + 16,
                glyph_height + 6,
                MENU.accent,
            );
            (MENU.background, MENU.accent)
        } else {
            let fg = if is_folder(row.entry) {
                MENU.accent
            } else {
                MENU.text
            };
            (fg, MENU.background)
        };
        let x = canvas.text(x, y, marker, fg, bg);
        canvas.text(x, y, name, fg, bg);
    }

    if *scroll + visible.len() < rows.len() {
        let y = list_top + visible.len() * pitch;
        if y + glyph_height <= bottom {
            canvas.text(arrow_x, y, "v", MENU.dim, MENU.background);
        }
    }
}
