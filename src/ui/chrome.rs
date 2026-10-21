//! The header and footer bars every graphical screen has.

use effie::protocols::BltPixel;

use super::surface::Surface;
use super::theme::{BAR_PADDING, MARGIN, Palette};

/// Draws the title bar and returns the y just below it.
pub fn header(surface: &mut impl Surface, palette: &Palette) -> usize {
    let width = surface.width();
    let height = surface.font().height + 2 * BAR_PADDING;
    surface.fill(0, 0, width, height, palette.bar);
    surface.fill(0, height, width, 1, palette.rule);
    let version = concat!("  v", env!("CARGO_PKG_VERSION"));
    spans(
        surface,
        palette,
        MARGIN,
        BAR_PADDING,
        &[("bread", palette.accent), (version, palette.dim)],
    );
    height + 1
}

/// Draws an empty footer bar. Returns the y of its top edge and the y for
/// text on it.
pub fn footer(surface: &mut impl Surface, palette: &Palette) -> (usize, usize) {
    let width = surface.width();
    let height = surface.font().height + 2 * BAR_PADDING;
    let top = surface.height().saturating_sub(height + 1);
    surface.fill(0, top, width, 1, palette.rule);
    surface.fill(0, top + 1, width, height, palette.bar);
    (top, top + 1 + BAR_PADDING)
}

/// Draws text in several colours on a bar, returning the x just past it.
pub fn spans(
    surface: &mut impl Surface,
    palette: &Palette,
    mut x: usize,
    y: usize,
    spans: &[(&str, BltPixel)],
) -> usize {
    for &(text, color) in spans {
        x = surface.text(x, y, text, color, palette.bar);
    }
    x
}
