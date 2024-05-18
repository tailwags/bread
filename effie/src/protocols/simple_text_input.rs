use crate::{Event, Guid, Protocol, Result, Status};

#[repr(C)]
pub struct SimpleTextInput {
    reset: unsafe extern "efiapi" fn(this: &Self, extended_verification: bool) -> Status,
    read_key_stroke: unsafe extern "efiapi" fn(this: &Self, key: *mut InputKey) -> Status,
    wait_for_key: Event,
}

#[repr(C)]
pub struct InputKey {
    pub scan_code: u16,
    pub unicode_char: u16,
}

impl Protocol for SimpleTextInput {
    const GUID: Guid = Guid::new(
        0x387477c1_u32.to_ne_bytes(),
        0x69c7_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl SimpleTextInput {
    pub fn reset(&self, extended_verification: bool) -> Result {
        unsafe { (self.reset)(self, extended_verification) }.as_result()
    }
}
