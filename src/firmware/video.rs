//! The preferred display mode.
//!
//! [`Preference`] is the `video` key of `bread.conf`, and [`apply`] switches the
//! GOP to the best mode for it. The menu and the framebuffer handed to the
//! kernel use the same mode, so this also picks what the kernel starts with on
//! x86-64. Only modes with a linear RGB or BGR framebuffer qualify, because the
//! kernel keeps using the framebuffer after boot; on x86-64 the mode also has
//! to fit in the kernel's `screen_info`.

use core::str::FromStr;

use effie::protocols::{GopPixelFormat, GraphicsOutput, GraphicsOutputModeInformation};

use crate::error::{Error, Result};

/// The `video` key: which display mode to prefer.
#[derive(Clone, Copy)]
pub enum Preference {
    /// The largest usable mode.
    Auto,
    /// The largest usable mode that fits in this size, or the closest one.
    Size { width: u16, height: u16 },
}

impl FromStr for Preference {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if value == "auto" {
            return Ok(Self::Auto);
        }

        // `screen_info` holds the resolution in 16 bits.
        let size = |field: &str| match field.parse::<u16>() {
            Ok(pixels) if pixels > 0 => Ok(pixels),
            _ => Err(Error::new(format_args!(
                "width and height must be between 1 and 65535, got '{value}'"
            ))),
        };

        let (width, height) = value.split_once('x').ok_or_else(|| {
            Error::new(format_args!(
                "expected 'auto' or '<width>x<height>', got '{value}'"
            ))
        })?;

        Ok(Self::Size {
            width: size(width)?,
            height: size(height)?,
        })
    }
}

/// Whether `info` describes a linear framebuffer, the only kind the kernel
/// keeps drawing on after boot.
fn linear(info: &GraphicsOutputModeInformation) -> bool {
    matches!(info.pixel_format, GopPixelFormat::RGB | GopPixelFormat::BGR)
}

/// Whether `info` describes a mode the kernel can keep using: linear, and small
/// enough to be described to it.
///
/// On x86-64 the kernel is handed the mode through `screen_info`, which stores
/// the width, height and line length in 16 bits: a larger mode would reach the
/// kernel with a truncated stride and it would draw over the wrong memory.
fn usable(info: &GraphicsOutputModeInformation) -> bool {
    if !linear(info) {
        return false;
    }
    // aarch64 is not handed a framebuffer description yet, so any size fits.
    if cfg!(not(target_arch = "x86_64")) {
        return true;
    }

    // The kernel draws with 4 bytes per pixel, which is what `screen_info`
    // stores as the line length.
    u16::try_from(info.horizontal_resolution).is_ok()
        && u16::try_from(info.vertical_resolution).is_ok()
        && u16::try_from(u64::from(info.pixels_per_scan_line) * 4).is_ok()
}

impl Preference {
    /// The mode to switch to, or `None` if the firmware offers no usable mode.
    fn choose(self, gop: &GraphicsOutput) -> Option<u32> {
        let area = |info: &GraphicsOutputModeInformation| {
            u64::from(info.horizontal_resolution) * u64::from(info.vertical_resolution)
        };
        // QueryMode fails for modes the firmware cannot describe, so skip them.
        let modes = || {
            (0..gop.current_mode().max_mode)
                .filter_map(|mode| Some((mode, gop.query_mode(mode).ok()?)))
                .filter(|(_, info)| usable(info))
        };

        let best = match self {
            Self::Auto => modes().max_by_key(|(_, info)| area(info)),
            Self::Size { width, height } => {
                let fits = |info: &GraphicsOutputModeInformation| {
                    info.horizontal_resolution <= u32::from(width)
                        && info.vertical_resolution <= u32::from(height)
                };
                // The exact mode is the largest one that fits, and staying
                // below the request also keeps the off-screen buffer small.
                modes()
                    .filter(|(_, info)| fits(info))
                    .max_by_key(|(_, info)| area(info))
                    // Nothing fits, so the screen ends up larger than asked
                    // for either way: the closest size beats the smallest one.
                    .or_else(|| {
                        let requested = u64::from(width) * u64::from(height);
                        modes().min_by_key(|(_, info)| area(info).abs_diff(requested))
                    })
            }
        };

        best.map(|(mode, _)| mode)
    }
}

/// Switches to the best mode for `preference` and returns the size of the mode
/// in use, or `None` if that mode has no linear framebuffer, in which case the
/// caller has to use the text console.
pub fn apply(gop: &mut GraphicsOutput, preference: Preference) -> Option<(usize, usize)> {
    if let Some(mode) = preference.choose(gop)
        && mode != gop.current_mode().mode
    {
        // A failure is not fatal: the mode in use may still be linear, and
        // effie's `set_mode` already logs the error.
        let _ = gop.set_mode(mode);
    }

    // The switch may have failed, so this is not necessarily the chosen mode.
    let mode = gop.current_mode();
    let info = gop.current_mode_info();
    if !linear(info) {
        log::warn!("the display mode has no linear framebuffer, using the text console");
        return None;
    }

    let (width, height) = (info.horizontal_resolution, info.vertical_resolution);
    log::info!("display mode {}: {width}x{height}", mode.mode);
    Some((width as usize, height as usize))
}
