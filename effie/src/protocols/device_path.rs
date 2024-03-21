use crate::{Guid, Protocol, Status};

// FIXME: EFI_DEVICE_PATH_PROTOCOL
#[repr(C)]
struct DevicePathRaw {}

#[repr(transparent)]
pub struct DevicePath {
    inner: *mut DevicePathRaw,
}
