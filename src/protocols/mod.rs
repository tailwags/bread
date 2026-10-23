//! Boot protocols: the ways bread can start something.
//!
//! Each protocol is one module owning its config entry (the keys between
//! `<keyword> <id>` and `end`) and how it boots. Adding a protocol means
//! adding a module and a variant to [`Protocol`] and [`Builder`]; the matches
//! below then show what else is missing. Code for one architecture goes in a
//! folder named after the protocol, like `linux/x86_64.rs`.

pub mod efi;
pub mod linux;

use crate::error::Result;

/// A bootable config entry.
pub enum Protocol {
    Linux(linux::Linux),
    Efi(efi::Efi),
}

impl Protocol {
    /// Boots the entry. Returns `Ok` when what it started gave control back
    /// (e.g. an EFI application exited), so the menu shows again.
    pub fn boot(&self) -> Result {
        match self {
            Protocol::Linux(entry) => linux::boot(entry),
            Protocol::Efi(entry) => efi::boot(entry),
        }
    }
}

/// Collects an entry's keys while the config is read.
pub enum Builder {
    Linux(linux::Builder),
    Efi(efi::Builder),
}

impl Builder {
    /// Starts an entry opened by `keyword`, or `None` if no protocol uses it.
    pub fn new(keyword: &str) -> Option<Self> {
        match keyword {
            "linux" => Some(Builder::Linux(Default::default())),
            "efi" => Some(Builder::Efi(Default::default())),
            _ => None,
        }
    }

    /// Takes one key of the entry. `Ok(false)` if the protocol has no such
    /// key.
    pub fn key(&mut self, key: &str, value: &str) -> Result<bool> {
        match self {
            Builder::Linux(builder) => builder.key(key, value),
            Builder::Efi(builder) => builder.key(key, value),
        }
    }

    /// Finishes the entry. Fails with the name of a missing key.
    pub fn finish(self) -> Result<Protocol, &'static str> {
        Ok(match self {
            Builder::Linux(builder) => Protocol::Linux(builder.finish()?),
            Builder::Efi(builder) => Protocol::Efi(builder.finish()?),
        })
    }
}
