use crate::tables::TableHeader;

#[repr(C)]
struct BootServicesRaw {
    hdr: TableHeader,
}

#[repr(transparent)]
pub struct BootServices {
    inner: *mut BootServicesRaw,
}
