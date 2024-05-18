use core::{ffi::c_void, mem::MaybeUninit};

use crate::{Guid, Result, Status};

#[repr(C)]
pub struct File {
    revision: u64,
    open: unsafe extern "efiapi" fn(
        this: &Self,
        new_handle: *mut *mut File,
        file_name: *const u16,
        open_mode: FileMode,
        attributes: u64,
    ) -> Status, // FIXME: type open_mode and attributes
    close: unsafe extern "efiapi" fn() -> Status,
    delete: unsafe extern "efiapi" fn() -> Status,
    read: unsafe extern "efiapi" fn(
        this: &Self,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    get_position: unsafe extern "efiapi" fn() -> Status,
    set_position: unsafe extern "efiapi" fn(this: &Self, position: u64) -> Status,
    get_info: unsafe extern "efiapi" fn(
        this: &Self,
        information_type: *mut Guid,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    set_info: unsafe extern "efiapi" fn() -> Status,
    flush: unsafe extern "efiapi" fn() -> Status,
    open_ex: unsafe extern "efiapi" fn() -> Status,
    read_ex: unsafe extern "efiapi" fn() -> Status,
    write_ex: unsafe extern "efiapi" fn() -> Status,
    flush_ex: unsafe extern "efiapi" fn() -> Status,
}

#[repr(transparent)]
pub struct FileMode(u64);

// TODO: modes combinations
impl FileMode {
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    pub const CREATE: Self = Self(0x8000000000000000);
}

impl File {
    pub fn open(&self, file_name: &[u16], open_mode: FileMode) -> Result<&File> {
        let mut file = MaybeUninit::<*mut File>::uninit();

        unsafe { (self.open)(self, file.as_mut_ptr(), file_name.as_ptr(), open_mode, 0) }
            .as_result_with(unsafe { &*file.assume_init() })
    }

    pub fn set_position(&self, position: u64) -> Result {
        unsafe { (self.set_position)(self, position) }.as_result()
    }

    pub fn read(&self, buf: &mut [u8]) -> Result {
        unsafe { (self.read)(self, &mut buf.len(), buf.as_mut_ptr().cast()) }.as_result()
    }

    // pub fn get_info(&self, buf: &mut [u8]) -> Result<>
}
