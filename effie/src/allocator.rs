use core::{alloc::GlobalAlloc, ptr::null_mut};

use crate::{system_table, tables::MemoryType};

#[repr(transparent)]
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let boot_services = system_table().boot_services();

        let size = layout.size();
        let align = layout.align();

        if align > 8 {
            // todo!() // FIXME: deal with pointer with bigger alignment
            null_mut()
        } else {
            if let Ok(ptr) = boot_services.allocate_pool(MemoryType::LOADER_DATA, size) {
                ptr.cast()
            } else {
                null_mut()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let boot_services = system_table().boot_services();

        let align = layout.align();

        if align > 8 {
            // todo!() // FIXME: deal with pointer with bigger alignment
        }

        // FIXME: can we deal with errors?
        let _ = boot_services.free_pool(ptr.cast());
    }
}
