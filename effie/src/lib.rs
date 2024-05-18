#![no_std]
#![feature(extended_varargs_abi_support)]

use core::mem::MaybeUninit;

mod allocator;
mod protocol;
mod status;
mod types;
mod util;

pub mod protocols;
pub mod tables;

pub use allocator::Allocator;
pub use protocol::Protocol;
pub use status::{Result, Status};
pub use types::*;

pub use uguid::Guid;

pub use effie_macros::w;

use tables::SystemTable;

static mut SYSTEM_TABLE: MaybeUninit<&SystemTable> = MaybeUninit::uninit();
static mut IMAGE_HANDLE: MaybeUninit<Handle> = MaybeUninit::uninit();

#[global_allocator]
static mut ALLOCATOR: Allocator = unsafe { Allocator::new() };

#[no_mangle]
extern "efiapi" fn efi_main(image_handle: Handle, system_table: &'static SystemTable) -> Status {
    extern "Rust" {
        fn main() -> Result;
    }

    unsafe { SYSTEM_TABLE.write(system_table) };
    unsafe { IMAGE_HANDLE.write(image_handle) };

    unsafe { ALLOCATOR.init(system_table.boot_services) };

    unsafe {
        if let Err(status) = main() {
            let _ = system_table.con_out().output_line(status.description());
            status
        } else {
            Status::SUCCESS
        }
    }
}

pub fn system_table() -> &'static SystemTable {
    unsafe { SYSTEM_TABLE.assume_init() }
}

pub fn image_handle() -> Handle {
    unsafe { IMAGE_HANDLE.assume_init() }
}
