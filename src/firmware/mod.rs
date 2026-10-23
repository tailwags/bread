//! Wrappers over the UEFI services bread uses, one module per service, for
//! code that is not tied to one boot protocol or screen.

pub mod fs;
pub mod memory;
pub mod reset;
pub mod video;
