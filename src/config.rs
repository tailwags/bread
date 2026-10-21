//! `\EFI\bread\bread.conf`, the boot menu written by the installer.
//!
//! Every line is a key, whitespace, then a value running to the end of the
//! line, so values never need quoting. `linux`, `efi` and `folder` open an
//! entry named by their value, closed by `end`; folders nest. Indentation is
//! cosmetic and lines starting with `#` are comments.
//!
//! ```text
//! timeout 5
//! firmware-setup yes
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

use crate::error::{Error, Result, ensure};
use crate::esp::{self, FileRef};

const PATH: &str = "\\EFI\\bread\\bread.conf";

pub struct Config {
    /// Seconds before the default entry boots on its own. `None` waits forever.
    pub timeout: Option<u32>,
    /// Id of the entry selected at startup. May name an entry inside a folder.
    pub default: Option<String>,
    /// PSF2 font for the menu, instead of the built-in one.
    pub font: Option<FileRef>,
    /// Offer rebooting into the firmware setup, if the firmware supports it.
    pub firmware_setup: bool,
    pub entries: Vec<Entry>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let data = esp::read_file(PATH)?;
        let text = str::from_utf8(&data).map_err(|e| Error::new(format_args!("{PATH}: {e}")))?;
        let mut parser = Parser {
            lines: text.lines(),
            line: 0,
        };

        let mut config = Config {
            timeout: None,
            default: None,
            font: None,
            firmware_setup: false,
            entries: Vec::new(),
        };
        while let Some((key, value)) = parser.next() {
            match key {
                "timeout" => {
                    let timeout = value.parse();
                    config.timeout =
                        Some(timeout.map_err(|e| parser.error(format_args!("timeout: {e}")))?);
                }
                "default" => config.default = Some(value.into()),
                "font" => config.font = Some(parser.file(value)?),
                "firmware-setup" => config.firmware_setup = parser.yes_no(value)?,
                "linux" | "efi" | "folder" => config.entries.push(parser.entry(key, value)?),
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
    Linux(Linux),
    Efi(Efi),
    Folder(Folder),
}

pub struct Linux {
    pub kernel: FileRef,
    /// Concatenated in order into a single initrd.
    pub initrd: Vec<FileRef>,
    pub cmdline: Option<String>,
}

/// A UEFI application to chainload.
pub struct Efi {
    pub path: FileRef,
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

    /// Parses the body of an entry opened by `kind` up to its `end`.
    fn entry(&mut self, kind: &str, id: &str) -> Result<Entry> {
        let mut name = None;
        let (mut kernel, mut initrd, mut cmdline) = (None, Vec::new(), None);
        let mut path = None;
        let (mut default, mut entries) = (None, Vec::new());

        loop {
            let Some((key, value)) = self.next() else {
                return Err(self.error(format_args!("'{id}' is missing its 'end'")));
            };
            match (kind, key) {
                (_, "end") => break,
                (_, "name") => name = Some(value.into()),
                ("linux", "kernel") => kernel = Some(self.file(value)?),
                ("linux", "initrd") => initrd.push(self.file(value)?),
                ("linux", "cmdline") => cmdline = Some(value.into()),
                ("efi", "path") => path = Some(self.file(value)?),
                ("folder", "default") => default = Some(value.into()),
                ("folder", "linux" | "efi" | "folder") => entries.push(self.entry(key, value)?),
                _ => self.unknown(key),
            }
        }

        let missing = |field| self.error(format_args!("'{id}' has no {field}"));
        let kind = match kind {
            "linux" => Kind::Linux(Linux {
                kernel: kernel.ok_or_else(|| missing("kernel"))?,
                initrd,
                cmdline,
            }),
            "efi" => Kind::Efi(Efi {
                path: path.ok_or_else(|| missing("path"))?,
            }),
            _ => Kind::Folder(Folder { default, entries }),
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
