#![no_main]
#![no_std]

extern crate alloc;

mod chainload;
mod config;
mod error;
mod esp;
mod firmware;
mod linux;
mod logfile;
mod panic;
mod ui;

use alloc::{format, string::String, vec::Vec};

use config::{Config, Kind};
use error::Error;
use log::LevelFilter;
use ui::{
    canvas::Canvas,
    error_screen::{self, Action},
    font::Font,
    menu::{Choice, Menu},
};

/// How long the screen for a broken menu waits before rebooting, since the
/// keyboard may be what broke.
const FATAL_TIMEOUT: u32 = 30;

#[unsafe(no_mangle)]
fn main() -> effie::Result {
    // Only fails if a logger is already registered, which cannot happen here.
    let _ = effie_log::builder()
        .max_level(if cfg!(debug_assertions) {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        })
        .uptime(true)
        // Saved to the ESP by the error screens and the panic handler.
        .history(32 * 1024)
        .init();
    log::info!("bread {}", env!("CARGO_PKG_VERSION"));

    let config = match Config::load() {
        Ok(config) => config,
        Err(error) => config_error(error),
    };
    let font = match &config.font {
        None => ui::font::EMBEDDED,
        Some(file) => Font::load(file).unwrap_or_else(|error| {
            log::warn!("{error}; using the built-in font");
            ui::font::EMBEDDED
        }),
    };
    let firmware_setup = config.firmware_setup && firmware::setup_supported();
    if config.firmware_setup && !firmware_setup {
        log::warn!("the firmware cannot reboot into its setup");
    }
    let mut actions = Vec::from([Action::BackToMenu, Action::Reboot]);
    if firmware_setup {
        actions.push(Action::FirmwareSetup);
    }

    let mut menu = Menu::new(&config, firmware_setup);
    let mut timeout = config.timeout;
    loop {
        // Acquired again on every pass: a chainloaded application may have
        // changed the display mode.
        let mut canvas = Canvas::acquire(font.clone());
        let choice = menu.run(canvas.as_mut(), timeout);
        // Frees the off-screen buffer (about 33 MB at 4K), which could sit
        // where the kernel wants to go. The display mode stays as it is, for
        // the kernel.
        drop(canvas);
        // Never count down again after a failure, or a broken default entry
        // would loop.
        timeout = None;

        let (title, error) = match choice {
            Ok(Choice::Boot(entry)) => {
                log::info!("booting '{}' ({})", entry.name, entry.id);
                let error = match &entry.kind {
                    Kind::Linux(linux) => match linux::prepare(linux) {
                        Ok(prepared) => prepared.start(),
                        Err(error) => error,
                    },
                    // Exiting with SUCCESS is not a failure: back to the menu.
                    Kind::Efi(efi) => match chainload::start(efi) {
                        Ok(()) => continue,
                        Err(error) => error,
                    },
                    Kind::Folder(_) => unreachable!("the menu never returns a folder"),
                };
                (format!("Could not boot '{}'", entry.name), error)
            }
            Ok(Choice::FirmwareSetup) => {
                let Err(error) = firmware::reboot_to_setup();
                (String::from(SETUP_FAILED), error)
            }
            // There is no menu to go back to.
            Err(error) => {
                let mut actions = Vec::from([Action::Reboot]);
                if firmware_setup {
                    actions.push(Action::FirmwareSetup);
                }
                recover(
                    &font,
                    "The boot menu failed",
                    error,
                    &actions,
                    Some(FATAL_TIMEOUT),
                );
                unreachable!("there is no menu to go back to")
            }
        };
        recover(&font, &title, error, &actions, None);
    }
}

const SETUP_FAILED: &str = "Could not reboot into the firmware setup";

/// Shows `error` and carries out the user's choice. Returns only for
/// [`Action::BackToMenu`].
fn recover(font: &Font, title: &str, error: Error, actions: &[Action], timeout: Option<u32>) {
    let (mut title, mut error) = (title, error);
    loop {
        match error_screen::show(font, title, &error, actions, timeout) {
            Action::BackToMenu => return,
            Action::Reboot => firmware::reboot(),
            Action::FirmwareSetup => {
                let Err(setup_error) = firmware::reboot_to_setup();
                (title, error) = (SETUP_FAILED, setup_error);
            }
        }
    }
}

/// There is no config, so the keys are fixed: R reboots, which most likely
/// comes back here, and F2 (when built with `config-error-firmware-setup`)
/// gives a way out that does not depend on the config.
fn config_error(error: Error) -> ! {
    let mut actions = Vec::from([Action::Reboot]);
    if cfg!(feature = "config-error-firmware-setup") && firmware::setup_supported() {
        actions.push(Action::FirmwareSetup);
    }
    let title = "bread.conf could not be loaded";
    recover(&ui::font::EMBEDDED, title, error, &actions, None);
    unreachable!("there is no menu to go back to")
}
