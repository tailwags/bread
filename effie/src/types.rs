use core::{ffi::c_void, ptr::null_mut};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Handle(*mut c_void);

impl Handle {
    pub const unsafe fn from_raw(raw: *mut c_void) -> Self {
        Self(raw)
    }

    pub const fn null() -> Self {
        Handle(null_mut())
    }
}

#[repr(transparent)]
pub struct Event(*mut c_void);
