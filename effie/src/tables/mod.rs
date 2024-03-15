mod boot_services;
mod runtime_services;
mod system;

pub use boot_services::*;
pub use runtime_services::*;
pub use system::*;

/// A 64-bit signature that identifies the type of table that follows. Unique signatures have been generated for
/// the EFI System Table, the EFI Boot Services Table, and the EFI Runtime Services Table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Signature(u64);

impl Signature {
    pub const SYSTEM_TABLE: Self = Self(0x5453595320494249);
}

/// Data structure that precedes all of the standard EFI table types.
#[repr(C)]
pub struct TableHeader {
    /// This signature that indentifies this table.
    pub signature: Signature,
    /// The revision of the EFI Specification to which this table conforms.
    pub revision: SystemTableRevision,
    /// The size, in bytes, of the entire table.
    pub size: u32,
    /// The 32-bit CRC for the entire table. This value is computed by setting this field to 0,
    /// and computing the 32-bit CRC for HeaderSize bytes.
    pub crc32: u32,
    /// Reserved field. Always set to 0
    __reserved: u32,
}
