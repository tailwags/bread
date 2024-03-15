#[repr(C)]
struct RuntimeServicesRaw {}

#[repr(transparent)]
pub struct RuntimeServices {
    inner: *mut RuntimeServicesRaw,
}
