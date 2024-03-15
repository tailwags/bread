#![no_std]

use core::ffi::c_void;

mod protocol;
mod status;

pub mod protocols;
pub mod tables;

pub use protocol::Protocol;
pub use status::Status;
pub use uguid::Guid;

#[doc(hidden)]
pub mod w;
#[doc(hidden)]
pub use w::*;

// pub type Handle = *mut c_void;
// pub type Event = *mut c_void;
#[repr(transparent)]
pub struct Handle(*mut c_void);
#[repr(transparent)]
pub struct Event(*mut c_void);
