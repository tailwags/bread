use core::ptr::NonNull;

#[repr(C)]
struct RuntimeServicesRaw {}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RuntimeServices {
    inner: NonNull<RuntimeServicesRaw>,
}
