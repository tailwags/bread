//! Colours and spacing shared by every screen.

use effie::protocols::BltPixel;

pub struct Palette {
    pub background: BltPixel,
    pub bar: BltPixel,
    pub rule: BltPixel,
    pub text: BltPixel,
    pub dim: BltPixel,
    pub accent: BltPixel,
    /// Text console attribute for ordinary text.
    pub console: usize,
    /// Text console attribute for titles.
    pub console_bright: usize,
}

/// The menu and the error screens.
pub const MENU: Palette = Palette {
    background: BltPixel::new(18, 12, 6),
    bar: BltPixel::new(26, 17, 8),
    rule: BltPixel::new(42, 28, 12),
    text: BltPixel::new(205, 185, 150),
    dim: BltPixel::new(105, 84, 58),
    accent: BltPixel::new(230, 170, 55),
    console: 0x07,
    console_bright: 0x0F,
};

/// The crash screen: bread itself failed.
pub const CRASH: Palette = Palette {
    background: BltPixel::new(40, 0, 0),
    bar: BltPixel::new(56, 6, 6),
    rule: BltPixel::new(84, 16, 16),
    text: BltPixel::new(255, 255, 255),
    dim: BltPixel::new(205, 150, 150),
    accent: BltPixel::new(255, 180, 160),
    // White on red.
    console: 0x4F,
    console_bright: 0x4F,
};

/// Margin around the content of every screen, in pixels.
pub const MARGIN: usize = 20;
/// Vertical padding around the text in the header and footer bars.
pub const BAR_PADDING: usize = 12;
