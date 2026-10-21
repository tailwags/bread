pub mod canvas;
pub mod error_screen;
pub mod font;
pub mod menu;
pub mod report;

use effie::protocols::BltPixel;

// The palette shared by the menu and the error screens.

pub const BACKGROUND: BltPixel = BltPixel::new(18, 12, 6);
pub const BAR: BltPixel = BltPixel::new(26, 17, 8);
pub const RULE: BltPixel = BltPixel::new(42, 28, 12);
pub const TEXT: BltPixel = BltPixel::new(205, 185, 150);
pub const DIM: BltPixel = BltPixel::new(105, 84, 58);
pub const ACCENT: BltPixel = BltPixel::new(230, 170, 55);

pub const MARGIN: usize = 20;
/// Vertical padding around the text in the header and footer bars.
pub const BAR_PADDING: usize = 12;
