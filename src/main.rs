#![no_main]
#![no_std]

use effie::{tables::SystemTable, Handle};
use windows_sys::w;

// KERNEL_PATH=boot:///efi/boot/vmlinuz
// MODULE_PATH=boot:///efi/boot/initrd.gz
// CMDLINE=loglevel=3

#[export_name = "efi_main"]
pub unsafe extern "efiapi" fn main(_image_handle: Handle, mut system_table: SystemTable) -> ! {
    system_table.con_out().clear_screen();

    system_table
        .con_out()
        .output_string(w!("I have become bread, bringer of gluten\n").cast_mut());

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
