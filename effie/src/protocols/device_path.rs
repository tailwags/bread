use crate::{Guid, Protocol};

// FIXME: EFI_DEVICE_PATH_PROTOCOL
#[repr(C)]
pub struct DevicePath {}

impl Protocol for DevicePath {
    const GUID: Guid = Guid::new(
        0x09576e91_u32.to_ne_bytes(),
        0x6d3f_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl DevicePath {
    // pub const fn null() -> Self {
    //     Self { inner: null_mut() }
    // }
}
