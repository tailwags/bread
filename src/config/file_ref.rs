//! Files named in the config.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt, str::FromStr};

use crate::error::{Error, Result, ensure};
use crate::firmware::fs;

/// A file on the ESP as named in the config, optionally pinned to its BLAKE3
/// digest with a `#blake3:<hex>` suffix.
pub struct FileRef {
    path: String,
    blake3: Option<blake3::Hash>,
}

impl FileRef {
    /// Reads the file and checks it against its digest, if it has one.
    pub fn read(&self) -> Result<Vec<u8>> {
        let data = fs::read_file(&self.path)?;

        if let Some(expected) = self.blake3 {
            let actual = blake3::hash(&data);
            ensure!(
                actual == expected,
                "{}: BLAKE3 mismatch\nexpected {expected}\ngot      {actual}",
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
