//! Files on the EFI System Partition bread was loaded from.

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{fmt, str::FromStr};

use effie::{
    WString,
    protocols::{FileHandle, FileMode, LoadedImage, SimpleFilesystem},
};

use crate::error::{Context, Error, Result, ensure};

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

/// A file on the ESP as named in the config, optionally pinned to its BLAKE3
/// digest with a `#blake3:<hex>` suffix.
pub struct FileRef {
    path: String,
    blake3: Option<blake3::Hash>,
}

impl FileRef {
    /// Reads the file and checks it against its digest, if it has one.
    pub fn read(&self) -> Result<Vec<u8>> {
        let data = read_file(&self.path)?;
        if let Some(expected) = self.blake3 {
            let actual = blake3::hash(&data);
            ensure!(
                actual == expected,
                "{}: BLAKE3 mismatch, expected {expected}, got {actual}",
                self.path,
            );
            log::info!("verified {}", self.path);
        }
        Ok(data)
    }
}

impl FromStr for FileRef {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let Some((path, fragment)) = value.split_once('#') else {
            return Ok(Self {
                path: value.to_string(),
                blake3: None,
            });
        };
        let Some(hex) = fragment.strip_prefix("blake3:") else {
            return Err(Error::new(format!(
                "{value}: unsupported digest '{fragment}'"
            )));
        };
        let hash = blake3::Hash::from_hex(hex)
            .map_err(|e| Error::new(format!("{value}: invalid BLAKE3 digest: {e}")))?;
        Ok(Self {
            path: path.to_string(),
            blake3: Some(hash),
        })
    }
}

impl fmt::Display for FileRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.path)
    }
}
