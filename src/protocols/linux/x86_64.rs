//! The x86-64 Linux boot protocol, entering through the kernel's 64-bit
//! entry point. See Documentation/arch/x86/boot.rst.

mod bootparam;

use core::{mem::size_of, ptr};

use effie::protocols::{GopPixelFormat, GraphicsOutput};
use effie::tables::{MemoryMap, MemoryType};

use self::bootparam::{BootParams, E820Entry, EfiInfo, ScreenInfo, SetupHeader};
use super::{Linux, load_initrd};
use crate::error::{Context, Result, ensure};
use crate::firmware::memory::{
    PAGE_SIZE, Placement, alloc_aligned, alloc_pages, exit_boot_services,
};

/// Offset of the setup header, both in the image and in `boot_params`.
const SETUP_HEADER: usize = 0x1f1;
/// "HdrS"
const MAGIC: u32 = 0x5372_6448;
/// 2.12 added `xloadflags`, which says whether there is a 64-bit entry point.
const MIN_VERSION: u16 = 0x020c;
const XLF_KERNEL_64: u16 = 1 << 0;
const XLF_CAN_BE_LOADED_ABOVE_4G: u16 = 1 << 1;
/// Offset of the 64-bit entry point in the protected-mode kernel.
const ENTRY_64: usize = 0x200;
const BELOW_4G: Placement = Placement::Below(u32::MAX as u64);
/// `type_of_loader` for boot loaders without an assigned id.
const UNDEFINED_LOADER: u8 = 0xff;

const VIDEO_TYPE_EFI: u8 = 0x70;
const VIDEO_CAPABILITY_SKIP_QUIRKS: u32 = 1 << 0;
const VIDEO_CAPABILITY_64BIT_BASE: u32 = 1 << 1;

pub struct Prepared {
    kernel_entry: usize,
    boot_params: &'static mut BootParams,
}

pub fn prepare(entry: &Linux, image: &[u8]) -> Result<Prepared> {
    let hdr = read_header(image)?;
    let above_4g = if hdr.xloadflags & XLF_CAN_BE_LOADED_ABOVE_4G != 0 {
        Placement::Anywhere
    } else {
        BELOW_4G
    };

    let kernel_entry = load_kernel(image, &hdr, above_4g)?;
    let initrd = load_initrd(&entry.initrd, Placement::Below(hdr.initrd_addr_max.into()))?;
    let cmdline = load_cmdline(entry.cmdline.as_deref().unwrap_or_default())?;

    let zero_page = alloc_pages(size_of::<BootParams>(), above_4g)
        .context("allocating memory for boot_params")?;
    // The setup header is copied as-is. It ends at 0x202 plus the byte at 0x201.
    let header_end =
        (0x202 + usize::from(image[0x201])).min(SETUP_HEADER + size_of::<SetupHeader>());
    zero_page[SETUP_HEADER..header_end].copy_from_slice(&image[SETUP_HEADER..header_end]);
    // SAFETY: the allocation is large enough, and `boot_params` is packed, so
    // any address is aligned for it.
    let bp = unsafe { &mut *zero_page.as_mut_ptr().cast::<BootParams>() };

    bp.hdr.type_of_loader = UNDEFINED_LOADER;
    bp.hdr.cmd_line_ptr = cmdline;
    if let Some(initrd) = initrd {
        bp.hdr.ramdisk_image = initrd.as_ptr() as u32;
        bp.hdr.ramdisk_size = initrd.len() as u32;
    }
    let st = effie::system_table();
    match st.find_acpi_rsdp() {
        Some(rsdp) => bp.acpi_rsdp_addr = rsdp,
        None => log::warn!("no ACPI RSDP in the UEFI configuration tables"),
    }
    describe_framebuffer(&mut bp.screen_info);

    log::info!(
        "kernel entry {kernel_entry:#x}, boot_params {:#x}",
        ptr::from_ref(bp) as usize
    );
    Ok(Prepared {
        kernel_entry,
        boot_params: bp,
    })
}

impl Prepared {
    /// Exits boot services and jumps to the kernel.
    pub fn start(self) -> ! {
        let bp = self.boot_params;
        let system_table = ptr::from_ref(effie::system_table()) as u64;
        let map = exit_boot_services();

        // Boot services are gone: no allocating and no logging from here on.
        fill_e820(bp, &map);
        let memmap = map.buffer.as_ptr() as u64;
        bp.efi_info = EfiInfo {
            efi_loader_signature: u32::from_le_bytes(*b"EL64"),
            efi_systab: system_table as u32,
            efi_systab_hi: (system_table >> 32) as u32,
            efi_memmap: memmap as u32,
            efi_memmap_hi: (memmap >> 32) as u32,
            efi_memmap_size: map.buffer.len() as u32,
            efi_memdesc_size: map.descriptor_size as u32,
            efi_memdesc_version: map.descriptor_version,
        };

        // SAFETY: the kernel, its boot information and the memory map are all in place.
        unsafe { enter_kernel(self.kernel_entry, ptr::from_mut(bp) as usize) }
    }
}

fn read_header(image: &[u8]) -> Result<SetupHeader> {
    ensure!(
        image.len() >= SETUP_HEADER + size_of::<SetupHeader>(),
        "kernel image is too small"
    );
    // SAFETY: in bounds, checked above.
    let hdr = unsafe {
        image
            .as_ptr()
            .add(SETUP_HEADER)
            .cast::<SetupHeader>()
            .read_unaligned()
    };

    let (magic, version, xloadflags) = (hdr.header, hdr.version, hdr.xloadflags);
    ensure!(magic == MAGIC, "kernel is not a bzImage");
    ensure!(
        version >= MIN_VERSION,
        "kernel boot protocol {version:#06x} is older than 2.12"
    );
    ensure!(
        xloadflags & XLF_KERNEL_64 != 0,
        "kernel has no 64-bit entry point"
    );
    log::info!("bzImage: boot protocol {version:#06x}");
    Ok(hdr)
}

/// Copies the protected-mode kernel to its preferred address or, if it is
/// relocatable, to any address with the alignment it asks for. Returns the
/// 64-bit entry point.
fn load_kernel(image: &[u8], hdr: &SetupHeader, above_4g: Placement) -> Result<usize> {
    let setup_sectors = match hdr.setup_sects {
        0 => 4,
        n => usize::from(n),
    };
    let payload = image.get((setup_sectors + 1) * 512..).unwrap_or_default();
    ensure!(
        !payload.is_empty(),
        "kernel image has no protected-mode code"
    );

    // `init_size` is what the kernel needs to decompress itself in place.
    let len = (hdr.init_size as usize).max(payload.len());
    let (preferred, align) = (hdr.pref_address, hdr.kernel_alignment as usize);
    let memory = match alloc_pages(len, Placement::At(preferred)) {
        Ok(memory) => memory,
        Err(_) if hdr.relocatable_kernel != 0 => {
            ensure!(
                align.is_power_of_two(),
                "kernel alignment {align:#x} is not a power of two"
            );
            alloc_aligned(len, align, above_4g).context("allocating memory for the kernel")?
        }
        Err(status) => {
            return Err(status).context(format_args!(
                "allocating memory for the kernel at {preferred:#x}"
            ));
        }
    };
    memory[..payload.len()].copy_from_slice(payload);
    Ok(memory.as_ptr() as usize + ENTRY_64)
}

/// Places the NUL-terminated command line below 4 GiB, since
/// `cmd_line_ptr` is 32 bits wide. Returns its address.
fn load_cmdline(cmdline: &str) -> Result<u32> {
    let memory = alloc_pages(cmdline.len() + 1, BELOW_4G)
        .context("allocating memory for the command line")?;
    memory[..cmdline.len()].copy_from_slice(cmdline.as_bytes());
    log::info!("command line: {cmdline}");
    Ok(memory.as_ptr() as u32)
}

/// Describes the current GOP framebuffer so the kernel can keep drawing on it.
fn describe_framebuffer(info: &mut ScreenInfo) {
    let boot_services = effie::system_table().boot_services();
    let Ok(gop) = boot_services.locate_protocol::<GraphicsOutput>() else {
        return;
    };
    let mode = gop.current_mode();
    let mode_info = gop.current_mode_info();
    let (red_pos, blue_pos) = match mode_info.pixel_format {
        GopPixelFormat::RGB => (0, 16),
        GopPixelFormat::BGR => (16, 0),
        // Not a linear framebuffer the kernel could use.
        _ => return,
    };
    let base = mode.frame_buffer_base;

    // `screen_info` describes the framebuffer with narrower fields than the GOP
    // mode does. A truncated line length or size would make the kernel draw
    // outside the framebuffer, so leave `screen_info` zeroed and the kernel on
    // the firmware's mode rather than hand it wrong values. `video::apply`
    // avoids such modes, but the firmware may have refused the switch.
    let width = mode_info.horizontal_resolution;
    let height = mode_info.vertical_resolution;
    let linelength = u64::from(mode_info.pixels_per_scan_line) * 4;
    let size = mode.frame_buffer_size;
    let (Ok(lfb_width), Ok(lfb_height), Ok(lfb_linelength), Ok(lfb_size)) = (
        u16::try_from(width),
        u16::try_from(height),
        u16::try_from(linelength),
        u32::try_from(size),
    ) else {
        log::warn!(
            "screen_info cannot describe the framebuffer: {width}x{height}, {linelength} byte lines, {size} bytes"
        );
        return;
    };

    info.orig_video_is_vga = VIDEO_TYPE_EFI;
    info.capabilities = VIDEO_CAPABILITY_64BIT_BASE | VIDEO_CAPABILITY_SKIP_QUIRKS;
    info.lfb_base = base as u32;
    info.ext_lfb_base = (base >> 32) as u32;
    info.lfb_size = lfb_size;
    info.lfb_width = lfb_width;
    info.lfb_height = lfb_height;
    info.lfb_depth = 32;
    info.lfb_linelength = lfb_linelength;
    info.red_size = 8;
    info.red_pos = red_pos;
    info.green_size = 8;
    info.green_pos = 8;
    info.blue_size = 8;
    info.blue_pos = blue_pos;
    info.rsvd_size = 8;
    info.rsvd_pos = 24;
    log::info!("framebuffer: {width}x{height} at {base:#x}");
}

/// Converts the UEFI memory map to the e820 table, merging adjacent ranges of
/// the same type.
fn fill_e820(bp: &mut BootParams, map: &MemoryMap) {
    let table = &mut bp.e820_table;
    let mut len: usize = 0;
    for descriptor in map.iter() {
        let addr = descriptor.physical_start.0;
        let size = descriptor.number_of_pages * PAGE_SIZE as u64;
        let type_ = e820_type(descriptor.ty);

        if let Some(last) = len.checked_sub(1).map(|i| &mut table[i])
            && { last.type_ } == type_
            && { last.addr } + { last.size } == addr
        {
            last.size += size;
            continue;
        }
        if len == table.len() {
            break;
        }
        table[len] = E820Entry { addr, size, type_ };
        len += 1;
    }
    bp.e820_entries = len as u8;
}

fn e820_type(ty: MemoryType) -> u32 {
    const RAM: u32 = 1;
    const RESERVED: u32 = 2;
    const ACPI: u32 = 3;
    const NVS: u32 = 4;
    const UNUSABLE: u32 = 5;
    const PMEM: u32 = 7;

    match ty {
        MemoryType::LOADER_CODE
        | MemoryType::LOADER_DATA
        | MemoryType::BOOT_SERVICES_CODE
        | MemoryType::BOOT_SERVICES_DATA
        | MemoryType::CONVENTIONAL_MEMORY => RAM,
        MemoryType::ACPIRECLAIM_MEMORY => ACPI,
        MemoryType::ACPIMEMORY_NVS => NVS,
        MemoryType::UNUSABLE_MEMORY => UNUSABLE,
        MemoryType::PERSISTENT_MEMORY => PMEM,
        _ => RESERVED,
    }
}

/// Jumps to the kernel's 64-bit entry point with `boot_params` in rsi.
///
/// The sysv64 ABI passes `entry` in rdi and `boot_params` in rsi, which is
/// exactly where `startup_64` expects the latter.
#[unsafe(naked)]
unsafe extern "sysv64" fn enter_kernel(entry: usize, boot_params: usize) -> ! {
    core::arch::naked_asm!("cli", "jmp rdi")
}
