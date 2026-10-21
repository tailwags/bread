//! Drawing on the GOP framebuffer.

use alloc::{vec, vec::Vec};

use effie::Protocol;
use effie::protocols::{BltPixel, GopPixelFormat, GraphicsOutput};

use super::font::Font;
use super::surface::Surface;

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
    /// Switches the display to its largest mode with a linear RGB or BGR
    /// framebuffer. Only those modes qualify because the kernel keeps using the
    /// framebuffer after boot. Returns `None` if there is no such mode.
    pub fn acquire(font: Font) -> Option<Self> {
        let boot_services = effie::system_table().boot_services();
        let mut gop = boot_services.locate_protocol::<GraphicsOutput>().ok()?;

        let best = (0..gop.current_mode().max_mode)
            .filter_map(|mode| Some((mode, gop.query_mode(mode).ok()?)))
            .filter(|(_, info)| {
                matches!(info.pixel_format, GopPixelFormat::RGB | GopPixelFormat::BGR)
            })
            .max_by_key(|(_, info)| {
                u64::from(info.horizontal_resolution) * u64::from(info.vertical_resolution)
            });
        let Some((mode, _)) = best else {
            log::warn!("no display mode with a linear framebuffer, using the text console");
            return None;
        };
        if gop.current_mode().mode != mode {
            gop.set_mode(mode).ok()?;
        }

        let info = gop.current_mode_info();
        let width = info.horizontal_resolution as usize;
        let height = info.vertical_resolution as usize;
        log::debug!("display mode {mode}: {width}x{height}");

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
