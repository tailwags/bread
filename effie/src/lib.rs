#![no_std]

use core::ffi::c_void;

mod protocol;
mod status;

pub mod protocols;
pub mod tables;

pub use protocol::Protocol;
pub use status::Status;
pub use uguid::Guid;

// pub type Handle = *mut c_void;
// pub type Event = *mut c_void;
#[repr(transparent)]
pub struct Handle(*mut c_void);
#[repr(transparent)]
pub struct Event(*mut c_void);
