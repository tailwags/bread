#![no_std]
#![feature(extended_varargs_abi_support)]
#![allow(unused)]

use core::ffi::c_void;

mod allocator;
mod protocol;
mod status;

pub mod protocols;
pub mod tables;

pub use allocator::Allocator;
pub use protocol::Protocol;
pub use status::{Result, Status};
pub use uguid::Guid;

pub use effie_macros::w;

// pub type Handle = *mut c_void;
// pub type Event = *mut c_void;
#[repr(transparent)]
pub struct Handle(*mut c_void);
#[repr(transparent)]
pub struct Event(*mut c_void);

pub(crate) unsafe fn u16_slice_from_ptr<'p>(ptr: *const u16) -> &'p [u16] {
    let mut len = 0;
    while *ptr.add(len) != 0u16 {
        len += 1
    }
    core::slice::from_raw_parts(ptr, len + 1)
}
