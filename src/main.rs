#![no_main]
#![no_std]

extern crate alloc;

use effie::{
    tables::{MemoryType, SystemTable},
    w, Allocator, Handle, Result, Status,
};

#[global_allocator]
static mut ALLOCATOR: Allocator = unsafe { Allocator::new() };

// KERNEL_PATH=boot:///efi/boot/vmlinuz
// MODULE_PATH=boot:///efi/boot/initrd.gz
// CMDLINE=loglevel=3

#[no_mangle]
pub extern "efiapi" fn efi_main(image_handle: Handle, system_table: SystemTable) -> Status {
    unsafe { ALLOCATOR.init(system_table.boot_services()) };

    match main(image_handle, system_table) {
        Ok(_) => Status::SUCCESS,
        Err(status) => status,
    }
}

fn main(_image_handle: Handle, system_table: SystemTable) -> Result {
    print_info(&system_table)?;

    boot()
}

fn print_info(system_table: &SystemTable) -> Result {
    let firmware_vendor = system_table.firmware_vendor();
    let con_out = system_table.con_out();

    con_out.clear_screen()?;
    con_out.output_string(w!("Firmware vendor: "))?;
    con_out.output_string(firmware_vendor)?;
    con_out.output_string(w!("\r\nUEFI version: "))?;
    con_out.output_string(system_table.revision().as_str())?;

    Ok(())
}

fn _load_kernel(system_table: &SystemTable) -> Result {
    let boot_services = system_table.boot_services();
    boot_services.allocate_pages_at_address(MemoryType::LOADER_DATA, 1, 0x800000.into())?;

    Ok(())
}

fn boot() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
    extern "C" {
        pub fn do_not_panic() -> !;
    }

    unsafe { do_not_panic() }
}
