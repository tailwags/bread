//! Starting another UEFI application from the ESP.

use crate::config::Efi;
use crate::error::{Context, Result};

/// Loads and starts the application. Returns when the application exits:
/// `Ok` if it exited with `SUCCESS` (e.g. the user left a UEFI shell).
pub fn start(entry: &Efi) -> Result {
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
