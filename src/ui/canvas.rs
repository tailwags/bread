//! Drawing on the GOP framebuffer.

use alloc::{vec, vec::Vec};

use effie::Protocol;
use effie::protocols::{BltPixel, GraphicsOutput};

use super::font::Font;
use super::surface::Surface;
use crate::firmware::video::{self, Preference};

/// Frames are drawn into an off-screen buffer in ordinary RAM, which is much
/// faster than drawing into the framebuffer, then copied to the screen with
/// one [`flush`](Self::flush).
pub struct Canvas {
    gop: Protocol<GraphicsOutput>,
    buffer: Vec<BltPixel>,
    pub width: usize,
    pub height: usize,
    pub font: Font,
}

impl Canvas {
    /// Switches the display to the mode chosen for `preference` and allocates
    /// the off-screen buffer for it. Returns `None` if there is no usable GOP
    /// mode, so the caller has to use the text console.
    pub fn acquire(font: Font, preference: Preference) -> Option<Self> {
        let boot_services = effie::system_table().boot_services();
        let mut gop = boot_services.locate_protocol::<GraphicsOutput>().ok()?;
        let (width, height) = video::apply(&mut gop, preference)?;

        Some(Self {
            gop,
            buffer: vec![BltPixel::default(); width * height],
            width,
            height,
            font,
        })
    }

    pub fn clear(&mut self, color: BltPixel) {
        self.buffer.fill(color);
    }

    /// Fills a rectangle, clipped to the screen.
    pub fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, color: BltPixel) {
        let x_end = x.saturating_add(w).min(self.width);
        let y_end = y.saturating_add(h).min(self.height);
        for row in y..y_end {
            let start = row * self.width;
            self.buffer[start + x.min(x_end)..start + x_end].fill(color);
        }
    }

    /// Draws `text` with its top-left corner at `(x, y)`, clipped to the
    /// screen. Returns the x coordinate just past the last character.
    pub fn text(
        &mut self,
        mut x: usize,
        y: usize,
        text: &str,
        fg: BltPixel,
        bg: BltPixel,
    ) -> usize {
        let rows = self.font.height.min(self.height.saturating_sub(y));
        for c in text.chars() {
            if x >= self.width {
                break;
            }
            let glyph = self.font.glyph(c);
            let columns = self.font.width.min(self.width - x);
            for gy in 0..rows {
                let start = (y + gy) * self.width + x;
                for (gx, pixel) in self.buffer[start..start + columns].iter_mut().enumerate() {
                    *pixel = if glyph.is_set(gx, gy) { fg } else { bg };
                }
            }
            x += self.font.width;
        }
        x
    }

    /// Copies the finished frame to the screen.
    pub fn flush(&mut self) {
        let _ = self.gop.blt_buffer_to_video(
            &self.buffer,
            0,
            0,
            0,
            0,
            self.width,
            self.height,
            self.width,
        );
    }
}

impl Surface for Canvas {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn font(&self) -> &Font {
        &self.font
    }

    fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, color: BltPixel) {
        Canvas::fill(self, x, y, width, height, color);
    }

    fn text(&mut self, x: usize, y: usize, text: &str, fg: BltPixel, bg: BltPixel) -> usize {
        Canvas::text(self, x, y, text, fg, bg)
    }
}
