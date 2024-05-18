#![no_main]
#![no_std]

extern crate alloc;

// use alloc::vec::Vec;
use effie::{
    protocols::{FileMode, LoadedImage, SimpleFilesystem},
    tables::{Signature, SystemTable},
    w, Result,
};

use core::mem::size_of;

use itoa::Integer;

#[cfg(target_arch = "x86_64")]
use linux_raw_sys::bootparam::setup_header as SetupHeader;
// #[cfg(target_arch = "aarch64")]
// use linux_raw_sys::image::arm64_image_header as ImageHeader;

const KERNEL_PATH: &[u16] = w!("\\efi\\boot\\vmlinuz");
const INITRAMFS_PATH: &[u16] = w!("\\efi\\boot\\initrd.gz");
const CMDLINE: &str = "loglevel=3";

// static mut SYSTEM_TABLE: MaybeUninit<&SystemTable> = MaybeUninit::uninit();

#[no_mangle]
fn main() -> Result {
    print_info()?;

    load_kernel()?;

    boot()
}

fn print_info() -> Result {
    let system_table = effie::system_table();

    let firmware_vendor = system_table.firmware_vendor();
    let con_out = system_table.con_out();

    con_out.clear_screen()?;
    con_out.output_string(w!("Firmware vendor: "))?;
    con_out.output_line(firmware_vendor)?;
    con_out.output_string(w!("UEFI version: "))?;
    con_out.output_line(system_table.revision().as_str())?;

    Ok(())
}

fn load_kernel() -> Result {
    let system_table = effie::system_table();
    let image_handle = effie::image_handle();

    let con_out = system_table.con_out();

    con_out.output_string(w!("Loading kernel\r\n"))?;

    let boot_services = system_table.boot_services();

    if boot_services.signature() == Signature::BOOT_SERVICES {
        con_out.output_string(w!("Signature matches\r\n"))?;
    }

    con_out.output_string(w!("Boot services\r\n"))?;

    let loaded_image = boot_services.open_protocol::<LoadedImage>(&image_handle, &image_handle)?;

    con_out.output_string(w!("Loaded image\r\n"))?;

    let file_system = system_table
        .boot_services()
        .open_protocol::<SimpleFilesystem>(loaded_image.device(), &image_handle)?;

    con_out.output_string(w!("Loaded system\r\n"))?;

    let volume = file_system.open_volume()?;
    let kernel_file = volume.open(KERNEL_PATH, FileMode::READ)?;

    con_out.output_string(w!("Wit\r\n"))?;

    // HACK: for some reason set_position doesn't seem to work, so we just read that amount of bytes into a temp buffer
    // kernel_file.set_position(0x01f1)?;
    let mut buf = [0u8; 0x01f1];
    kernel_file.read(&mut buf)?;

    let mut buf = [0u8; size_of::<SetupHeader>()];
    kernel_file.read(&mut buf)?;

    let setup_header: SetupHeader = unsafe { core::ptr::read(buf.as_ptr().cast()) };

    if setup_header.header == 0x53726448 {
        con_out.output_string(w!("Kernel signatures matches\r\n"))?;
    } else {
        con_out.output_string(w!("Kernel signatures doesn't match\r\n"))?;
    }

    Ok(())
}

fn boot() -> ! {
    loop {}
}

fn _print_num<I: Integer>(system_table: &SystemTable, i: I) -> Result {
    let mut buffer = itoa::Buffer::new();
    let printed = buffer.format(i);

    _print_utf8(system_table, printed)?;

    Ok(())
}

fn _print_utf8(system_table: &SystemTable, string: &str) -> Result {
    for c in string.encode_utf16() {
        system_table.con_out().output_string(&[c, 0])?;
    }

    system_table.con_out().output_string(w!("\r\n"))?;

    Ok(())
}

#[panic_handler]
fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
    extern "C" {
        pub fn do_not_panic() -> !;
    }

    unsafe { do_not_panic() }
}
