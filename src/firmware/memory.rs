//! Memory handed on to what bread boots, and leaving boot services.

use core::mem::ManuallyDrop;

use effie::tables::{MemoryMap, MemoryType, PhysicalAddress};

pub const PAGE_SIZE: usize = 4096;

/// Where [`alloc_pages`] may put memory.
#[derive(Clone, Copy)]
pub enum Placement {
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
pub fn alloc_pages(len: usize, placement: Placement) -> effie::Result<&'static mut [u8]> {
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
pub fn alloc_aligned(
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

/// Stops serial logging and exits boot services. After this nothing may
/// allocate, and the returned map must not be dropped. Nothing can be undone
/// at this point, so a failure panics.
pub fn exit_boot_services() -> ManuallyDrop<MemoryMap> {
    log::info!("exiting boot services");

    // Logging must not call firmware between fetching the memory map and
    // exiting, or the map would change and the exit fail.
    effie_log::detach();

    // SAFETY: bread is its own image, and nothing is used after this that
    // needs boot services.
    match unsafe { effie::exit_boot_services() } {
        Ok(map) => map,
        Err(status) => panic!("exiting boot services: {status}"),
    }
}
