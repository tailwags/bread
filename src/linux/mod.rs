//! Booting Linux through its native boot protocol rather than the EFI stub.
//! bread places the kernel, initrd and boot information in memory itself,
//! exits boot services and jumps to the kernel.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
use aarch64 as arch;
#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
use x86_64 as arch;

use alloc::vec::Vec;
use core::mem::ManuallyDrop;

use effie::tables::{MemoryMap, MemoryType, PhysicalAddress};

use crate::config::Linux;
use crate::error::{Context, Result};
use crate::esp::FileRef;

const PAGE_SIZE: usize = 4096;

/// Loads and starts the kernel. Returns only if something fails before the
/// kernel has control.
pub fn boot(entry: &Linux) -> Result<!> {
    let kernel = entry.kernel.read()?;
    arch::boot(entry, &kernel)
}

/// Where [`alloc_pages`] may put memory.
#[derive(Clone, Copy)]
enum Placement {
    Anywhere,
    /// Ending at or below this address.
    #[cfg_attr(
        target_arch = "aarch64",
        expect(dead_code, reason = "used on x86-64 only")
    )]
    Below(u64),
    #[cfg_attr(
        target_arch = "aarch64",
        expect(dead_code, reason = "used on x86-64 only")
    )]
    At(u64),
}

/// Allocates zeroed memory to hand to the kernel. It is never freed.
fn alloc_pages(len: usize, placement: Placement) -> effie::Result<&'static mut [u8]> {
    let boot_services = effie::system_table().boot_services();
    let pages = len.div_ceil(PAGE_SIZE).max(1);
    let ty = MemoryType::LOADER_DATA;
    let address = match placement {
        Placement::Anywhere => boot_services.allocate_any_pages(ty, pages)?.0,
        Placement::Below(max) => {
            boot_services
                .allocate_max_address_pages(ty, pages, PhysicalAddress(max))?
                .0
        }
        Placement::At(address) => {
            boot_services.allocate_pages_at_address(ty, pages, PhysicalAddress(address))?;
            address
        }
    };
    // SAFETY: the firmware just gave us these pages, and UEFI identity-maps
    // memory. They are zeroed before any reference to them exists.
    unsafe {
        let start = address as *mut u8;
        start.write_bytes(0, pages * PAGE_SIZE);
        Ok(core::slice::from_raw_parts_mut(start, len))
    }
}

/// Like [`alloc_pages`], with the start aligned to `align`, a power of two.
fn alloc_aligned(
    len: usize,
    align: usize,
    placement: Placement,
) -> effie::Result<&'static mut [u8]> {
    // Firmware allocations are only page-aligned, so allocate extra and skip
    // ahead to the boundary.
    let memory = alloc_pages(len + align, placement)?;
    let skip = memory.as_ptr().align_offset(align);
    Ok(&mut memory[skip..skip + len])
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

/// Stops serial logging and exits boot services. After this nothing may
/// allocate, and the returned map must not be dropped.
fn exit_boot_services() -> Result<ManuallyDrop<MemoryMap>> {
    log::info!("exiting boot services");
    // Logging must not call firmware between fetching the memory map and
    // exiting, or the map would change and the exit fail.
    effie_log::detach();
    // SAFETY: bread is its own image, and nothing is used after this that
    // needs boot services.
    unsafe { effie::exit_boot_services() }.context("exiting boot services")
}
