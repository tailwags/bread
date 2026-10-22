//! PC Screen Font v2 (PSF2) bitmap fonts.
//!
//! A PSF2 file is a 32-byte header of little-endian `u32`s (magic, version,
//! header size, flags, glyph count, bytes per glyph, height, width), then the
//! glyph bitmaps: one bit per pixel, most significant bit first, each row
//! padded to whole bytes. When flag bit 0 is set, a unicode table follows. For
//! each glyph it lists the UTF-8 characters the glyph draws, then optional
//! `0xFE`-prefixed multi-character sequences, then `0xFF`.
//!
//! Fonts borrow their data and never allocate, so the panic handler can draw
//! with them too.

use crate::config::FileRef;
use crate::error::{Error, Result};

const MAGIC: usize = 0x864a_b572;
const HAS_UNICODE_TABLE: usize = 1;
const SEQUENCE: u8 = 0xFE;
const END: u8 = 0xFF;

/// The built-in font (Cozette 6×13, Latin-1 subset), checked at compile time.
/// It has no unicode table: glyph indices are code points U+0000–U+00FF.
pub const EMBEDDED: Font = match Font::parse(include_bytes!("cozette.psf")) {
    Ok(font) => font,
    Err(_) => panic!("the embedded font is not a valid PSF2 font"),
};

#[derive(Clone)]
pub struct Font {
    /// Glyph cell width in pixels.
    pub width: usize,
    /// Glyph cell height in pixels.
    pub height: usize,
    row_bytes: usize,
    glyph_bytes: usize,
    glyph_count: usize,
    glyphs: &'static [u8],
    /// Empty when the font has no unicode table.
    unicode: &'static [u8],
    /// Glyph index of each ASCII character, so most lookups skip the table.
    ascii: [u16; 128],
}

impl Font {
    /// Reads a PSF2 font from the ESP.
    pub fn load(file: &FileRef) -> Result<Self> {
        // The font is used until the kernel takes over, so its data can live forever.
        let data = file.read()?.leak();
        Self::parse(data).map_err(|e| Error::new(format_args!("{file}: {e}")))
    }

    pub const fn parse(bytes: &'static [u8]) -> core::result::Result<Self, &'static str> {
        if bytes.len() < 32 {
            return Err("truncated header");
        }
        let header_size = u32_at(bytes, 8);
        let flags = u32_at(bytes, 12);
        let glyph_count = u32_at(bytes, 16);
        let glyph_bytes = u32_at(bytes, 20);
        let height = u32_at(bytes, 24);
        let width = u32_at(bytes, 28);
        let row_bytes = width.div_ceil(8);

        if u32_at(bytes, 0) != MAGIC || u32_at(bytes, 4) != 0 {
            return Err("not a PSF2 font");
        }
        if width == 0 || height == 0 || glyph_count == 0 || glyph_count > u16::MAX as usize {
            return Err("unsupported glyph size or count");
        }
        if header_size < 32 || glyph_bytes < row_bytes * height {
            return Err("inconsistent header");
        }
        let Some(glyphs_end) = header_size.checked_add(glyph_count * glyph_bytes) else {
            return Err("inconsistent header");
        };
        if bytes.len() < glyphs_end {
            return Err("truncated glyph data");
        }

        let (head, table) = bytes.split_at(glyphs_end);
        let (_, glyphs) = head.split_at(header_size);
        let unicode: &[u8] = if flags & HAS_UNICODE_TABLE != 0 {
            table
        } else {
            &[]
        };

        Ok(Self {
            width,
            height,
            row_bytes,
            glyph_bytes,
            glyph_count,
            glyphs,
            unicode,
            ascii: ascii_table(unicode, glyph_count),
        })
    }

    /// Returns the glyph for `c`, or glyph 0 if the font does not have one.
    pub fn glyph(&self, c: char) -> Glyph {
        let start = self.index(c) * self.glyph_bytes;
        Glyph {
            bits: &self.glyphs[start..start + self.row_bytes * self.height],
            row_bytes: self.row_bytes,
        }
    }

    /// Width of `text` in pixels.
    pub fn text_width(&self, text: &str) -> usize {
        text.chars().count() * self.width
    }

    fn index(&self, c: char) -> usize {
        if let Some(&index) = self.ascii.get(c as usize) {
            return index.into();
        }
        if self.unicode.is_empty() {
            return if (c as usize) < self.glyph_count {
                c as usize
            } else {
                0
            };
        }
        let mut buf = [0; 4];
        let needle = c.encode_utf8(&mut buf).as_bytes();
        self.unicode
            .split(|&b| b == END)
            .take(self.glyph_count)
            .position(|entry| {
                let single = entry.split(|&b| b == SEQUENCE).next().unwrap_or_default();
                // UTF-8 is self-synchronising, so a byte match is a character match.
                single.windows(needle.len()).any(|bytes| bytes == needle)
            })
            .unwrap_or(0)
    }
}

/// One character's bitmap.
pub struct Glyph {
    bits: &'static [u8],
    row_bytes: usize,
}

impl Glyph {
    /// Whether the pixel at column `x`, row `y` of the cell is drawn.
    pub fn is_set(&self, x: usize, y: usize) -> bool {
        self.bits[y * self.row_bytes + x / 8] & (0x80 >> (x % 8)) != 0
    }
}

const fn u32_at(bytes: &[u8], offset: usize) -> usize {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]) as usize
}

const fn ascii_table(unicode: &[u8], glyph_count: usize) -> [u16; 128] {
    let mut table = [0u16; 128];

    if unicode.is_empty() {
        // Without a unicode table, glyph indices are code points.
        let mut c = 0;
        while c < 128 && c < glyph_count {
            table[c] = c as u16;
            c += 1;
        }
        return table;
    }

    // In UTF-8 a byte below 0x80 is always a complete ASCII character, so the
    // table can be scanned byte by byte. The first glyph listing a character wins.
    let mut mapped = [false; 128];
    let (mut i, mut glyph, mut in_sequence) = (0, 0, false);
    while i < unicode.len() && glyph < glyph_count {
        let b = unicode[i];
        if b == END {
            glyph += 1;
            in_sequence = false;
        } else if b == SEQUENCE {
            in_sequence = true;
        } else if b < 0x80 && !in_sequence && !mapped[b as usize] {
            table[b as usize] = glyph as u16;
            mapped[b as usize] = true;
        }
        i += 1;
    }
    table
}
