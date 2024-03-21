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
    pub const BOOT_SERVICES: Self = Self(0x56524553544f4f42);
}

/// Data structure that precedes all of the standard EFI table types.
#[repr(C)]
pub struct TableHeader {
    /// This signature that indentifies this table.
    pub signature: Signature,
    /// The revision of the EFI Specification to which this table conforms.
    pub revision: SpecificationRevision,
    /// The size, in bytes, of the entire table.
    pub size: u32,
    /// The 32-bit CRC for the entire table. This value is computed by setting this field to 0,
    /// and computing the 32-bit CRC for HeaderSize bytes.
    pub crc32: u32,
    /// Reserved field. Always set to 0
    __reserved: u32,
}

/// TODO
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SpecificationRevision(u32);

impl SpecificationRevision {
    pub const EFI_2_100: Self = Self((2 << 16) | (100));
    pub const EFI_2_90: Self = Self((2 << 16) | (90));
    pub const EFI_2_80: Self = Self((2 << 16) | (80));
    pub const EFI_2_70: Self = Self((2 << 16) | (70));
    pub const EFI_2_60: Self = Self((2 << 16) | (60));
    pub const EFI_2_50: Self = Self((2 << 16) | (50));
    pub const EFI_2_40: Self = Self((2 << 16) | (40));
    pub const EFI_2_31: Self = Self((2 << 16) | (31));
    pub const EFI_2_30: Self = Self((2 << 16) | (30));
    pub const EFI_2_20: Self = Self((2 << 16) | (20));
    pub const EFI_2_10: Self = Self((2 << 16) | (10));
    pub const EFI_2_00: Self = Self((2 << 16) | (00));
    pub const EFI_1_10: Self = Self((1 << 16) | (10));
    pub const EFI_1_02: Self = Self((1 << 16) | (02));
    pub const EFI: Self = Self::EFI_2_100;
}
