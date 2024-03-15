#![no_main]
#![no_std]

use r_efi::efi::{self, Status};
use windows_sys::w;

mod system;

use system::SystemTable;

// KERNEL_PATH=boot:///efi/boot/vmlinuz
// MODULE_PATH=boot:///efi/boot/initrd.gz
// CMDLINE=loglevel=3

#[export_name = "efi_main"]
pub unsafe extern "efiapi" fn main(image_handle: efi::Handle, system_table: SystemTable) -> Status {
    system_table.print(w!("I have become bread, bringer of gluten").cast_mut());
    let _volume = system_table.get_volume(image_handle);

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
