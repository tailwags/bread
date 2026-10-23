//! The x86 zero page (`struct boot_params`) and setup header, from
//! arch/x86/include/uapi/asm/bootparam.h. Layout per
//! Documentation/arch/x86/zero-page.rst and boot.rst: fields are only ever
//! added in padding, so offsets are stable. Parts bread has no use for (legacy
//! BIOS data) are kept as opaque bytes. Field names follow the kernel.

use core::mem::{offset_of, size_of};

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct BootParams {
    pub screen_info: ScreenInfo,
    pub apm_bios_info: [u8; 0x14],
    pub _pad2: [u8; 4],
    pub tboot_addr: u64,
    pub ist_info: [u8; 0x10],
    pub acpi_rsdp_addr: u64,
    pub _pad3: [u8; 8],
    pub hd0_info: [u8; 16],
    pub hd1_info: [u8; 16],
    pub sys_desc_table: [u8; 0x10],
    pub olpc_ofw_header: [u8; 0x10],
    pub ext_ramdisk_image: u32,
    pub ext_ramdisk_size: u32,
    pub ext_cmd_line_ptr: u32,
    pub _pad4: [u8; 112],
    pub cc_blob_address: u32,
    pub edid_info: [u8; 0x80],
    pub efi_info: EfiInfo,
    pub alt_mem_k: u32,
    pub scratch: u32,
    pub e820_entries: u8,
    pub eddbuf_entries: u8,
    pub edd_mbr_sig_buf_entries: u8,
    pub kbd_status: u8,
    pub secure_boot: u8,
    pub _pad5: [u8; 2],
    pub sentinel: u8,
    pub _pad6: [u8; 1],
    pub hdr: SetupHeader,
    pub _pad7: [u8; 36],
    pub edd_mbr_sig_buffer: [u32; 16],
    pub e820_table: [E820Entry; 128],
    pub _pad8: [u8; 48],
    /// 6 × `struct edd_info`.
    pub eddbuf: [u8; 6 * 82],
    pub _pad9: [u8; 276],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ScreenInfo {
    pub orig_x: u8,
    pub orig_y: u8,
    pub ext_mem_k: u16,
    pub orig_video_page: u16,
    pub orig_video_mode: u8,
    pub orig_video_cols: u8,
    pub flags: u8,
    pub unused2: u8,
    pub orig_video_ega_bx: u16,
    pub unused3: u16,
    pub orig_video_lines: u8,
    /// `orig_video_isVGA`: the video type (`VIDEO_TYPE_*`).
    pub orig_video_is_vga: u8,
    pub orig_video_points: u16,
    pub lfb_width: u16,
    pub lfb_height: u16,
    pub lfb_depth: u16,
    pub lfb_base: u32,
    pub lfb_size: u32,
    pub cl_magic: u16,
    pub cl_offset: u16,
    pub lfb_linelength: u16,
    pub red_size: u8,
    pub red_pos: u8,
    pub green_size: u8,
    pub green_pos: u8,
    pub blue_size: u8,
    pub blue_pos: u8,
    pub rsvd_size: u8,
    pub rsvd_pos: u8,
    pub vesapm_seg: u16,
    pub vesapm_off: u16,
    pub pages: u16,
    pub vesa_attributes: u16,
    pub capabilities: u32,
    pub ext_lfb_base: u32,
    pub _reserved: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiInfo {
    pub efi_loader_signature: u32,
    pub efi_systab: u32,
    pub efi_memdesc_size: u32,
    pub efi_memdesc_version: u32,
    pub efi_memmap: u32,
    pub efi_memmap_size: u32,
    pub efi_systab_hi: u32,
    pub efi_memmap_hi: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SetupHeader {
    pub setup_sects: u8,
    pub root_flags: u16,
    pub syssize: u32,
    pub ram_size: u16,
    pub vid_mode: u16,
    pub root_dev: u16,
    pub boot_flag: u16,
    pub jump: u16,
    pub header: u32,
    pub version: u16,
    pub realmode_swtch: u32,
    pub start_sys_seg: u16,
    pub kernel_version: u16,
    pub type_of_loader: u8,
    pub loadflags: u8,
    pub setup_move_size: u16,
    pub code32_start: u32,
    pub ramdisk_image: u32,
    pub ramdisk_size: u32,
    pub bootsect_kludge: u32,
    pub heap_end_ptr: u16,
    pub ext_loader_ver: u8,
    pub ext_loader_type: u8,
    pub cmd_line_ptr: u32,
    pub initrd_addr_max: u32,
    pub kernel_alignment: u32,
    pub relocatable_kernel: u8,
    pub min_alignment: u8,
    pub xloadflags: u16,
    pub cmdline_size: u32,
    pub hardware_subarch: u32,
    pub hardware_subarch_data: u64,
    pub payload_offset: u32,
    pub payload_length: u32,
    pub setup_data: u64,
    pub pref_address: u64,
    pub init_size: u32,
    pub handover_offset: u32,
    pub kernel_info_offset: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct E820Entry {
    pub addr: u64,
    pub size: u64,
    pub type_: u32,
}

// Offsets from zero-page.rst and boot.rst.
const _: () = {
    assert!(size_of::<BootParams>() == 0x1000);
    assert!(size_of::<ScreenInfo>() == 0x40);
    assert!(size_of::<EfiInfo>() == 0x20);
    assert!(size_of::<E820Entry>() == 20);
    assert!(offset_of!(BootParams, acpi_rsdp_addr) == 0x070);
    assert!(offset_of!(BootParams, cc_blob_address) == 0x13c);
    assert!(offset_of!(BootParams, efi_info) == 0x1c0);
    assert!(offset_of!(BootParams, e820_entries) == 0x1e8);
    assert!(offset_of!(BootParams, sentinel) == 0x1ef);
    assert!(offset_of!(BootParams, hdr) == 0x1f1);
    assert!(offset_of!(BootParams, e820_table) == 0x2d0);
    assert!(offset_of!(BootParams, eddbuf) == 0xd00);
    assert!(offset_of!(BootParams, hdr.type_of_loader) == 0x210);
    assert!(offset_of!(BootParams, hdr.cmd_line_ptr) == 0x228);
    assert!(offset_of!(BootParams, hdr.xloadflags) == 0x236);
    assert!(offset_of!(BootParams, hdr.pref_address) == 0x258);
    assert!(offset_of!(BootParams, hdr.kernel_info_offset) == 0x268);
    assert!(offset_of!(BootParams, screen_info.orig_video_is_vga) == 0x0f);
    assert!(offset_of!(BootParams, screen_info.capabilities) == 0x36);
    assert!(offset_of!(BootParams, screen_info.ext_lfb_base) == 0x3a);
};
