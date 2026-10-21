//! The boot menu: a list of entries in which folders expand in place.
//!
//! [`tree`] decides what is listed and selected, [`graphics`] and [`text`]
//! draw it, and [`Menu::run`] turns keys and the countdown into a choice.

mod graphics;
mod text;
mod tree;

use effie::protocols::InputKey;

use self::tree::{Tree, is_folder};
use crate::config::{Config, Entry};
use crate::error::{Context, Result};
use crate::ui::canvas::Canvas;
use crate::ui::input;

/// What the user picked.
pub enum Choice<'a> {
    /// Never a folder.
    Boot(&'a Entry),
    FirmwareSetup,
}

/// The menu's state outlives [`Menu::run`], so after a failed boot the menu
/// comes back as the user left it.
pub struct Menu<'a> {
    tree: Tree<'a>,
    firmware_setup: bool,
    /// First row shown when the list is taller than the screen.
    scroll: usize,
}

impl<'a> Menu<'a> {
    /// `firmware_setup` enables F2 for rebooting into the firmware setup.
    pub fn new(config: &'a Config, firmware_setup: bool) -> Self {
        Self {
            tree: Tree::new(&config.entries, config.default.as_deref()),
            firmware_setup,
            scroll: 0,
        }
    }

    /// Returns what to do: the entry the user picks, the selected entry once
    /// `timeout` seconds pass without a key, or the firmware setup on F2.
    pub fn run(
        &mut self,
        mut canvas: Option<&mut Canvas>,
        timeout: Option<u32>,
    ) -> Result<Choice<'a>> {
        let st = effie::system_table();
        let boot_services = st.boot_services();
        let mut con_in = st.con_in().context("opening the console input")?;
        let mut countdown = timeout;

        loop {
            let rows = self.tree.rows();
            let selected = self.tree.selected;
            let entry = rows[selected].entry;
            match &mut canvas {
                Some(canvas) => graphics::draw(
                    canvas,
                    &rows,
                    selected,
                    &mut self.scroll,
                    countdown,
                    self.firmware_setup,
                ),
                None => text::print(&rows, selected, countdown, self.firmware_setup),
            }

            let key = match countdown {
                Some(0) if !is_folder(entry) => {
                    log::info!("timeout, booting '{}'", entry.name);
                    return Ok(Choice::Boot(entry));
                }
                Some(0) => {
                    countdown = None;
                    continue;
                }
                Some(seconds) => match input::within_a_second(&mut con_in, boot_services) {
                    Some(key) => {
                        countdown = None;
                        key
                    }
                    None => {
                        countdown = Some(seconds - 1);
                        continue;
                    }
                },
                None => input::wait(&mut con_in, boot_services)?,
            };

            match key.scan_code {
                InputKey::SCAN_UP => self.tree.selected = selected.saturating_sub(1),
                InputKey::SCAN_DOWN => self.tree.selected = (selected + 1).min(rows.len() - 1),
                InputKey::SCAN_F2 if self.firmware_setup => return Ok(Choice::FirmwareSetup),
                InputKey::SCAN_NULL if key.unicode_char == u16::from(b'\r') => {
                    if is_folder(entry) {
                        self.tree.toggle(entry);
                    } else {
                        log::info!("selected '{}'", entry.name);
                        return Ok(Choice::Boot(entry));
                    }
                }
                _ => {}
            }
        }
    }
}
