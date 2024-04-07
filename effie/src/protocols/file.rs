use crate::{Guid, Protocol, Status};

#[repr(C)]
struct FileRaw {}

#[repr(transparent)]
pub struct File {
    inner: *mut FileRaw,
}
