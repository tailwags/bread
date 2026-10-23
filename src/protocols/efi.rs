//! Chainloading another UEFI application from the ESP.

use crate::{
    config::FileRef,
    error::{Context, Result},
};

/// An `efi` entry.
pub struct Efi {
    pub path: FileRef,
}

/// The keys of an `efi` entry, as the config is read.
#[derive(Default)]
pub struct Builder {
    path: Option<FileRef>,
}

impl Builder {
    pub fn key(&mut self, key: &str, value: &str) -> Result<bool> {
        match key {
            "path" => self.path = Some(value.parse()?),
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn finish(self) -> Result<Efi, &'static str> {
        Ok(Efi {
            path: self.path.ok_or("path")?,
        })
    }
}

/// Loads and starts the application. Returns when the application exits:
/// `Ok` if it exited with `SUCCESS` (e.g. the user left a UEFI shell).
pub fn boot(entry: &Efi) -> Result {
    let image = entry.path.read()?;
    let boot_services = effie::system_table().boot_services();

    let handle = boot_services
        .load_image(effie::image_handle(), &image)
        .context(format_args!("loading {}", entry.path))?;
    drop(image);

    log::info!("starting {}", entry.path);
    boot_services
        .start_image(handle)
        .context(format_args!("running {}", entry.path))?;
    log::info!("{} exited", entry.path);
    Ok(())
}
