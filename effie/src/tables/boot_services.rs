#[repr(C)]
struct BootServicesRaw {}

#[repr(transparent)]
pub struct BootServices {
    inner: *mut BootServicesRaw,
}
