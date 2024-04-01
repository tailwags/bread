use core::{
    alloc::GlobalAlloc,
    ptr::null_mut,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::tables::{BootServices, BootServicesRaw, MemoryType};

#[repr(transparent)]
pub struct Allocator {
    inner: AtomicPtr<BootServicesRaw>,
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let boot_services = self.boot_services();

        let size = layout.size();
        let align = layout.align();

        if align > 8 {
            todo!() // FIXME: deal with pointer with bigger alignment
        } else {
            if let Ok(ptr) = boot_services.allocate_pool(MemoryType::LOADER_DATA, size) {
                ptr.cast()
            } else {
                null_mut()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let boot_services = self.boot_services();

        let align = layout.align();

        if align > 8 {
            todo!() // FIXME: deal with pointer with bigger alignment
        }

        boot_services.free_pool(ptr.cast());
    }
}

impl Allocator {
    pub const unsafe fn new() -> Self {
        Self {
            inner: AtomicPtr::new(null_mut()),
        }
    }

    pub unsafe fn init(&mut self, boot_services: &BootServices) {
        self.inner.store(boot_services.as_raw(), Ordering::Relaxed);
    }

    fn boot_services(&self) -> BootServices {
        unsafe { BootServices::from_raw(self.inner.load(Ordering::Acquire)) }
    }
}
