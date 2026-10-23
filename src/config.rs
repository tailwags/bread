//! `\EFI\bread\bread.conf`, the boot menu written by the installer.
//!
//! Every line is a key, whitespace, then a value running to the end of the
//! line, so values never need quoting. `linux`, `efi` and `folder` open an
//! entry named by their value, closed by `end`; folders nest. Indentation is
//! cosmetic and lines starting with `#` are comments. The keys inside a
//! `linux` or `efi` entry belong to its [protocol](crate::protocols).
//!
//! ```text
//! timeout 5
//! firmware-setup yes
//! video 1920x1080
//! default nixos
//! folder nixos
//!   name NixOS
//!   linux gen-42
//!     name Generation 42
//!     kernel \EFI\bread\kernels\bzImage#blake3:<hex>
//!     initrd \EFI\bread\kernels\initrd
//!     cmdline init=/nix/store/...-init quiet
//!   end
//! end
//! ```

use alloc::{string::String, vec::Vec};
use core::{fmt, str::Lines};

mod file_ref;

pub use file_ref::FileRef;

use crate::error::{Error, Result, ensure};
use crate::firmware::fs;
use crate::firmware::video::Preference;
use crate::protocols::{Builder, Protocol};

const PATH: &str = "\\EFI\\bread\\bread.conf";

pub struct Config {
    /// Seconds before the default entry boots on its own. `None` waits forever.
    pub timeout: Option<u32>,
    /// Id of the entry selected at startup. May name an entry inside a folder.
    pub default: Option<String>,
    /// PSF2 font for the menu, instead of the built-in one.
    pub font: Option<FileRef>,
    /// Resolution to prefer for the menu and the kernel framebuffer.
    pub video: Preference,
    /// Offer rebooting into the firmware setup, if the firmware supports it.
    pub firmware_setup: bool,
    pub entries: Vec<Entry>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let data = fs::read_file(PATH)?;
        let text = str::from_utf8(&data).map_err(|e| Error::new(format_args!("{PATH}: {e}")))?;

        let mut parser = Parser {
            lines: text.lines(),
            line: 0,
        };

        let mut config = Config {
            timeout: None,
            default: None,
            font: None,
            video: Preference::Auto,
            firmware_setup: false,
            entries: Vec::new(),
        };

        while let Some((key, value)) = parser.next() {
            match key {
                "timeout" => {
                    let timeout = value.parse().ok();
                    config.timeout =
                        Some(timeout.ok_or_else(|| parser.error("timeout: expected seconds"))?);
                }
                "default" => config.default = Some(value.into()),
                "font" => config.font = Some(parser.file(value)?),
                "video" => config.video = value.parse().map_err(|e| parser.error(e))?,
                "firmware-setup" => config.firmware_setup = parser.yes_no(value)?,
                _ if opens_entry(key) => config.entries.push(parser.entry(key, value)?),
                "end" => return Err(parser.error("'end' outside an entry")),
                _ => parser.unknown(key),
            }
        }

        ensure!(!config.entries.is_empty(), "{PATH} has no entries");

        Ok(config)
    }
}

pub struct Entry {
    pub id: String,
    pub name: String,
    pub kind: Kind,
}

pub enum Kind {
    Boot(Protocol),
    Folder(Folder),
}

pub struct Folder {
    /// Id of the entry to select when the menu opens this folder by default.
    pub default: Option<String>,
    pub entries: Vec<Entry>,
}

struct Parser<'a> {
    lines: Lines<'a>,
    /// Number of the last line read, for error messages.
    line: usize,
}

impl<'a> Parser<'a> {
    /// Returns the next line as `(key, value)`, skipping blanks and comments.
    fn next(&mut self) -> Option<(&'a str, &'a str)> {
        for line in self.lines.by_ref() {
            self.line += 1;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            return Some(match line.split_once([' ', '\t']) {
                Some((key, value)) => (key, value.trim_start()),
                None => (line, ""),
            });
        }

        None
    }

    /// Parses the body of an entry opened by `keyword` up to its `end`.
    /// Folders are read here, everything else by its protocol.
    fn entry(&mut self, keyword: &str, id: &str) -> Result<Entry> {
        let mut name = None;
        let mut protocol = Builder::new(keyword);
        let (mut default, mut entries) = (None, Vec::new());

        loop {
            let Some((key, value)) = self.next() else {
                return Err(self.error(format_args!("'{id}' is missing its 'end'")));
            };

            match (&mut protocol, key) {
                (_, "end") => break,
                (_, "name") => name = Some(value.into()),
                (Some(protocol), _) => {
                    if !protocol.key(key, value).map_err(|e| self.error(e))? {
                        self.unknown(key);
                    }
                }
                (None, "default") => default = Some(value.into()),
                (None, _) if opens_entry(key) => entries.push(self.entry(key, value)?),
                (None, _) => self.unknown(key),
            }
        }

        let missing = |field| self.error(format_args!("'{id}' has no {field}"));

        let kind = match protocol {
            Some(protocol) => Kind::Boot(protocol.finish().map_err(missing)?),
            None => Kind::Folder(Folder { default, entries }),
        };

        Ok(Entry {
            id: id.into(),
            name: name.ok_or_else(|| missing("name"))?,
            kind,
        })
    }

    fn file(&self, value: &str) -> Result<FileRef> {
        value.parse().map_err(|e| self.error(e))
    }

    fn yes_no(&self, value: &str) -> Result<bool> {
        match value {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(self.error(format_args!("expected 'yes' or 'no', got '{value}'"))),
        }
    }

    /// Unknown keys are skipped so an older bread can read a newer config.
    fn unknown(&self, key: &str) {
        log::warn!("{PATH}:{}: ignoring unknown key '{key}'", self.line);
    }

    fn error(&self, message: impl fmt::Display) -> Error {
        Error::new(format_args!("{PATH}:{}: {message}", self.line))
    }
}

/// Whether `key` starts a nested entry: a folder or a protocol's entry.
fn opens_entry(key: &str) -> bool {
    key == "folder" || Builder::new(key).is_some()
}
