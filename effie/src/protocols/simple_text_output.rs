use crate::{Guid, Protocol, Result, Status};

#[repr(C)]
struct SimpleTextOutputRaw {
    reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: bool) -> Status,
    output_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> Status,
    test_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> Status,
    query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> Status,
    set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: usize) -> Status,
    set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attribute: usize) -> Status,
    clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, column: usize, row: usize) -> Status,
    enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> Status,
    mode: *mut SimpleTextOutputMode,
}

#[repr(C)]
struct SimpleTextOutputMode {
    max_mode: i32,
    mode: i32,
    attribute: i32,
    cursor_column: i32,
    cursor_row: i32,
    cursor_visible: bool,
}

#[repr(transparent)]
pub struct SimpleTextOutput {
    inner: *mut SimpleTextOutputRaw,
}

impl Protocol for SimpleTextOutput {
    const GUID: Guid = Guid::new(
        0x387477c2_u32.to_ne_bytes(),
        0x69c7_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl SimpleTextOutput {
    pub fn output_string(&self, string: &[u16]) -> Result {
        unsafe { ((*self.inner).output_string)(self.inner, string.as_ptr()) }.as_result()
    }

    pub fn clear_screen(&self) -> Result {
        unsafe { ((*self.inner).clear_screen)(self.inner) }.as_result()
    }
}
