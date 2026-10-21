//! Something to draw on: the buffered [`Canvas`](super::canvas::Canvas) the
//! menu uses, or [`Direct`], which draws straight to the screen without
//! allocating, for the reports.

use effie::protocols::{BltPixel, GraphicsOutput};

use super::font::Font;

pub trait Surface {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn font(&self) -> &Font;
    fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, color: BltPixel);
    /// Draws `text` with its top-left corner at `(x, y)` and returns the x
    /// just past it.
    fn text(&mut self, x: usize, y: usize, text: &str, fg: BltPixel, bg: BltPixel) -> usize;
}

/// Pixels in the buffer a glyph is drawn into before it goes to the screen.
const CELL: usize = 2048;

/// Draws on the screen directly, one glyph at a time.
pub struct Direct<'a> {
    gop: &'a mut GraphicsOutput,
    font: &'a Font,
    width: usize,
    height: usize,
}

impl<'a> Direct<'a> {
    pub fn new(gop: &'a mut GraphicsOutput, font: &'a Font) -> Self {
        let info = gop.current_mode_info();
        let width = info.horizontal_resolution as usize;
        let height = info.vertical_resolution as usize;
        Self {
            gop,
            font,
            width,
            height,
        }
    }
}

impl Surface for Direct<'_> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn font(&self) -> &Font {
        self.font
    }

    fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, color: BltPixel) {
        let _ = self.gop.blt_video_fill(color, x, y, width, height);
    }

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
