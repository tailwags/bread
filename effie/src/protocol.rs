pub use crate::Guid;

pub trait Protocol {
    const GUID: Guid;
}
