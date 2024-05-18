use effie_macros::w;

use crate::{Guid, Protocol, Result, Status};

#[repr(C)]
pub struct SimpleTextOutput {
    reset: unsafe extern "efiapi" fn(this: &Self, extended_verification: bool) -> Status,
    output_string: unsafe extern "efiapi" fn(this: &Self, string: *const u16) -> Status,
    test_string: unsafe extern "efiapi" fn(this: &Self, string: *const u16) -> Status,
    query_mode: unsafe extern "efiapi" fn(
        this: &Self,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> Status,
    set_mode: unsafe extern "efiapi" fn(this: &Self, mode_number: usize) -> Status,
    set_attribute: unsafe extern "efiapi" fn(this: &Self, attribute: usize) -> Status,
    clear_screen: unsafe extern "efiapi" fn(this: &Self) -> Status,
    set_cursor_position:
        unsafe extern "efiapi" fn(this: &Self, column: usize, row: usize) -> Status,
    enable_cursor: unsafe extern "efiapi" fn(this: &Self, visible: bool) -> Status,
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
        unsafe { (self.output_string)(self, string.as_ptr()) }.as_result()
    }

    pub fn output_line(&self, string: &[u16]) -> Result {
        self.output_string(string)?;
        self.output_string(w!("\r\n"))
    }

    pub fn clear_screen(&self) -> Result {
        unsafe { (self.clear_screen)(self) }.as_result()
    }
}
