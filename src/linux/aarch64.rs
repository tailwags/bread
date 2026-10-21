//! The arm64 Linux boot protocol. See Documentation/arch/arm64/booting.rst.
//!
//! Incomplete: the command line and initrd have to be added to the device
//! tree's `/chosen` node, which bread does not do yet.

use core::{arch::asm, mem::size_of};

use linux_raw_sys::image::arm64_image_header;

use super::{Placement, alloc_aligned, exit_boot_services, load_initrd};
use crate::config::Linux;
use crate::error::{Context, Error, Result, ensure};

/// "ARM\x64"
const MAGIC: u32 = 0x644d_5241;
/// The image goes `text_offset` bytes past a 2 MiB aligned base.
const BASE_ALIGN: usize = 2 << 20;

pub struct Prepared {
    kernel_entry: usize,
    fdt: usize,
}

pub fn prepare(entry: &Linux, image: &[u8]) -> Result<Prepared> {
    ensure!(
        image.len() >= size_of::<arm64_image_header>(),
        "kernel image is too small"
    );
    // SAFETY: in bounds, checked above.
    let hdr = unsafe { image.as_ptr().cast::<arm64_image_header>().read_unaligned() };
    ensure!(
        u32::from_le(hdr.magic) == MAGIC,
        "kernel is not an arm64 Image"
    );

    let text_offset = u64::from_le(hdr.text_offset) as usize;
    // `image_size` includes the BSS. Kernels before 3.17 leave it zero.
    let image_size = (u64::from_le(hdr.image_size) as usize).max(image.len());
    let base = alloc_aligned(text_offset + image_size, BASE_ALIGN, Placement::Anywhere)
        .context("allocating memory for the kernel")?;
    let kernel = &mut base[text_offset..];
    kernel[..image.len()].copy_from_slice(image);
    clean_dcache(kernel);

    if let Some(initrd) = load_initrd(&entry.initrd, Placement::Anywhere)? {
        clean_dcache(initrd);
        log::warn!(
            "the initrd is not passed to the kernel: device tree patching is not implemented"
        );
    }
    if entry.cmdline.is_some() {
        log::warn!(
            "the command line is not passed to the kernel: device tree patching is not implemented"
        );
    }

    let fdt = effie::system_table()
        .find_dtb()
        .ok_or_else(|| Error::new("no device tree in the UEFI configuration tables"))?;
    let kernel_entry = kernel.as_ptr() as usize;
    log::info!("kernel entry {kernel_entry:#x}, device tree {fdt:#x}");
    Ok(Prepared { kernel_entry, fdt })
}

impl Prepared {
    /// Exits boot services and jumps to the kernel.
    pub fn start(self) -> ! {
        let _map = exit_boot_services();

        // SAFETY: the kernel is in place and written back to memory.
        unsafe { enter_kernel(self.kernel_entry, self.fdt) }
    }
}

/// Cleans and invalidates `data` in the data cache, so the kernel still sees
/// it after turning the caches off.
fn clean_dcache(data: &[u8]) {
    let ctr: u64;
    // SAFETY: reading CTR_EL0 has no side effects.
    unsafe { asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack)) };
    // DminLine: log2 of the smallest data cache line size, in 4-byte words.
    let line = 4 << ((ctr >> 16) & 0xf);

    let start = data.as_ptr() as usize & !(line - 1);
    let end = data.as_ptr() as usize + data.len();
    for address in (start..end).step_by(line) {
        // SAFETY: cache maintenance on memory we own.
        unsafe { asm!("dc civac, {}", in(reg) address, options(nostack)) };
    }
    // SAFETY: barriers only.
    unsafe { asm!("dsb ish", "isb", options(nostack)) };
}

/// Turns off the MMU and caches and jumps to the kernel with the device tree
/// in x0.
///
/// The boot protocol wants x0 = device tree address and x1-x3 = 0. The
/// `extern "C"` ABI brings `entry` in x0 and `fdt` in x1. x4 holds the entry
/// point for the final branch and x5 is scratch for SCTLR_EL1.
///
/// Assumes EL1. Firmware running at EL2 needs the same done to SCTLR_EL2.
#[unsafe(naked)]
unsafe extern "C" fn enter_kernel(entry: usize, fdt: usize) -> ! {
    core::arch::naked_asm!(
        "mov x4, x0",
        "mov x0, x1",
        "mov x1, xzr",
        "mov x2, xzr",
        "mov x3, xzr",
        "mrs x5, sctlr_el1",
        "bic x5, x5, #(1 << 0)",  // M: MMU off
        "bic x5, x5, #(1 << 2)",  // C: data cache off
        "bic x5, x5, #(1 << 12)", // I: instruction cache off
        "msr sctlr_el1, x5",
        "isb",
        "br x4",
    )
}
