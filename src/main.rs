#![no_main]
#![no_std]

extern crate alloc;

mod chainload;
mod config;
mod error;
mod esp;
mod linux;
mod panic;
mod ui;

use config::{Config, Kind};
use error::{Result, ensure};
use log::LevelFilter;
use ui::{canvas::Canvas, font::Font};

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
        // Saved to the ESP by the panic handler.
        .history(32 * 1024)
        .init();
    log::info!("bread {}", env!("CARGO_PKG_VERSION"));

    // The panic handler shows the error and saves the log.
    let Err(error) = run();
    panic!("{error}")
}

fn run() -> Result<!> {
    let config = Config::load()?;
    ensure!(!config.entries.is_empty(), "bread.conf has no entries");

    let font = match &config.font {
        None => ui::font::EMBEDDED,
        Some(file) => Font::load(file).unwrap_or_else(|error| {
            log::warn!("{error}; using the built-in font");
            ui::font::EMBEDDED
        }),
    };
    let entry = ui::menu::run(&config, Canvas::acquire(font))?;

    log::info!("booting '{}' ({})", entry.name, entry.id);
    match &entry.kind {
        Kind::Linux(linux) => linux::boot(linux),
        Kind::Efi(efi) => chainload::start(efi),
        Kind::Folder(_) => unreachable!("the menu never returns a folder"),
    }
}
