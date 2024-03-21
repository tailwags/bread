#![no_main]
#![no_std]

use effie::{tables::SystemTable, w, Handle};

// KERNEL_PATH=boot:///efi/boot/vmlinuz
// MODULE_PATH=boot:///efi/boot/initrd.gz
// CMDLINE=loglevel=3

#[export_name = "efi_main"]
pub unsafe extern "efiapi" fn main(_image_handle: Handle, system_table: SystemTable) -> ! {
    let firmware_vendor = system_table.firmware_vendor();
    let con_out = system_table.con_out();

    con_out.clear_screen();
    con_out.output_string(w!("Found firmware vendor: "));
    con_out.output_string(firmware_vendor);

    let boot_services = system_table.boot_services();

    boot()
}

pub unsafe fn boot() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
    extern "C" {
        pub fn do_not_panic() -> !;
    }

    unsafe { do_not_panic() }
}
