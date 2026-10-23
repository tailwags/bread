//! The menu on the text console, when there is no usable graphics mode.

use alloc::format;

use super::tree::{Row, is_folder};
use crate::ui::{console, theme::MENU};

/// Black on light grey, for the selected entry.
const INVERTED: usize = 0x70;

/// Prints the header, the entries and the key hints.
pub fn print(rows: &[Row], selected: usize, countdown: Option<u32>, firmware_setup: bool) {
    let Ok(mut out) = effie::system_table().con_out() else {
        return;
    };
    let _ = out.enable_cursor(false);
    let _ = out.clear_screen();
    let mut line = |attribute: usize, text: &str| {
        let _ = out.set_attribute(attribute);
        console::line(&mut out, text);
    };

    line(
        MENU.console_bright,
        concat!("  bread  v", env!("CARGO_PKG_VERSION")),
    );
    line(MENU.console, "");
    for (i, row) in rows.iter().enumerate() {
        let marker = match (is_folder(row.entry), row.open) {
            (true, false) => "  [+]",
            (true, true) => "  [-]",
            _ => "",
        };
        let name = &row.entry.name;
        let indent = 2 * (row.depth + 1);
        let text = format!("{:indent$}{name}{marker}", "");
        let attribute = if i == selected {
            INVERTED
        } else {
            MENU.console
        };
        line(attribute, &text);
    }
    line(MENU.console, "");
    line(MENU.console, "  UP / DOWN  navigate    ENTER  boot / open");
    if firmware_setup {
        line(MENU.console, "  F2  firmware setup");
    }
    if let Some(seconds) = countdown {
        line(MENU.console, &format!("  Booting in {seconds}s"));
    }
}
