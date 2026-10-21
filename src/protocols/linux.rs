//! Booting Linux through its native boot protocol rather than the EFI stub.
//! bread places the kernel, initrd and boot information in memory itself,
//! exits boot services and jumps to the kernel.
//!
//! Booting is split at the point of no return: [`prepare`] does everything
//! that can fail and still go back to the menu, and [`Prepared::start`]
//! everything after. The arch-specific halves live in `linux/<arch>.rs`.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
use aarch64 as arch;
#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
use x86_64 as arch;

use alloc::{string::String, vec::Vec};

use arch::Prepared;

use crate::config::FileRef;
use crate::error::{Context, Result};
use crate::firmware::memory::{Placement, alloc_pages};

/// A `linux` entry.
pub struct Linux {
    pub kernel: FileRef,
    /// Concatenated in order into a single initrd.
    pub initrd: Vec<FileRef>,
    pub cmdline: Option<String>,
}

/// The keys of a `linux` entry, as the config is read.
#[derive(Default)]
pub struct Builder {
    kernel: Option<FileRef>,
    initrd: Vec<FileRef>,
    cmdline: Option<String>,
}

impl Builder {
    pub fn key(&mut self, key: &str, value: &str) -> Result<bool> {
        match key {
            "kernel" => self.kernel = Some(value.parse()?),
            "initrd" => self.initrd.push(value.parse()?),
            "cmdline" => self.cmdline = Some(value.into()),
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn finish(self) -> core::result::Result<Linux, &'static str> {
        Ok(Linux {
            kernel: self.kernel.ok_or("kernel")?,
            initrd: self.initrd,
            cmdline: self.cmdline,
        })
    }
}

/// Boots the kernel. Returns only if preparing it failed.
pub fn boot(entry: &Linux) -> Result {
    prepare(entry)?.start()
}

/// Reads and verifies the kernel and initrd, and places them and the boot
/// information in memory. Then [`Prepared::start`] boots.
///
/// Every file is read and verified before any page is allocated, so the
/// common failures (a missing file, a digest mismatch) leak nothing. Pages
/// allocated before a later failure (a bad image, an allocation that does not
/// fit) are never freed. That is harmless while the only ways on from a
/// failure are the menu and a reboot.
fn prepare(entry: &Linux) -> Result<Prepared> {
    let kernel = entry.kernel.read()?;
    arch::prepare(entry, &kernel)
}

/// Reads the initrd parts into one block of memory. Returns `None` if there
/// are no parts.
fn load_initrd(parts: &[FileRef], placement: Placement) -> Result<Option<&'static [u8]>> {
    if parts.is_empty() {
        return Ok(None);
    }
    let parts = parts
        .iter()
        .map(FileRef::read)
        .collect::<Result<Vec<_>>>()?;
    let len = parts.iter().map(Vec::len).sum();
    let initrd = alloc_pages(len, placement).context("allocating memory for the initrd")?;

    let mut offset = 0;
    for part in &parts {
        initrd[offset..offset + part.len()].copy_from_slice(part);
        offset += part.len();
    }
    log::info!("initrd: {len} bytes at {:#x}", initrd.as_ptr() as usize);
    Ok(Some(initrd))
}
