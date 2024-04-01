use core::{
    alloc::GlobalAlloc,
    ptr::{null_mut, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::tables::BootServices;

pub struct Allocator {
    boot_services: AtomicPtr<BootServices>,
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}

impl Allocator {
    pub const unsafe fn new() -> Self {
        Self {
            boot_services: AtomicPtr::new(null_mut()),
        }
    }

    pub unsafe fn init(&self, boot_services: NonNull<BootServices>) {
        self.boot_services
            .store(boot_services.as_ptr(), Ordering::Relaxed)
    }
}
