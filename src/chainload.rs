//! Starting another UEFI application from the ESP.

use crate::config::Efi;
use crate::error::{Context, Error, Result};

/// Loads and starts the application. Returns only on failure, including when
/// the application exits back to bread.
pub fn start(entry: &Efi) -> Result<!> {
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

    Err(Error::new(format_args!("{} exited", entry.path)))
}
