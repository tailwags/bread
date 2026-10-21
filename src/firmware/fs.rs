//! The file system bread was loaded from: the ESP holding `bread.conf`, the
//! kernels and the log.

use alloc::{vec, vec::Vec};

use effie::{
    WString,
    protocols::{FileHandle, FileMode, LoadedImage, SimpleFilesystem},
};

use crate::error::{Context, Result, ensure};

/// Opens the root directory of the volume bread was loaded from.
///
/// Needs boot services, so it must run before ExitBootServices.
pub fn root() -> effie::Result<FileHandle> {
    let boot_services = effie::system_table().boot_services();
    let handle = effie::image_handle();
    let image = boot_services.open_protocol::<LoadedImage>(&handle, &handle)?;
    let mut fs = boot_services.open_protocol::<SimpleFilesystem>(image.device(), &handle)?;
    fs.open_volume()
}

/// Reads a whole file, given its backslash-separated path.
pub fn read_file(path: &str) -> Result<Vec<u8>> {
    let mut file = root()
        .and_then(|mut root| root.open(&WString::from(path), FileMode::READ))
        .context(format_args!("opening {path}"))?;

    let size = file
        .get_info()
        .context(format_args!("reading {path}"))?
        .file_size() as usize;

    let mut data = vec![0u8; size];
    let read = file
        .read(&mut data)
        .context(format_args!("reading {path}"))?;

    ensure!(read == size, "reading {path}: got {read} of {size} bytes");

    log::debug!("read {path} ({size} bytes)");
    Ok(data)
}
