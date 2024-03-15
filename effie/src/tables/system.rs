use core::ffi::c_void;

use crate::{
    protocols::{SimpleTextInput, SimpleTextOutput},
    tables::{BootServices, RuntimeServices, TableHeader},
    u16_slice_from_ptr, Guid, Handle,
};

/// TODO
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SystemTableRevision(u32);

impl SystemTableRevision {
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

/// Contains pointers to the runtime and boot services tables.
#[repr(C)]
struct SystemTableRaw {
    /// TODO
    hdr: TableHeader,
    /// A pointer to a null terminated string that identifies the vendor that produces the system firmware for the platform
    firmware_vendor: *const u16,
    /// A firmware vendor specific value that identifies the revision of the system firmware for the platform.
    firmware_version: u32,
    /// The handle for the active console input device.
    console_in_handle: Handle,
    /// A pointer to the EFI_SIMPLE_TEXT_INPUT_PROTOCOL interface that is associated with ConsoleInHandle.
    con_in: SimpleTextInput,
    console_out_handle: Handle,
    con_out: SimpleTextOutput,
    standard_error_handler: Handle,
    std_err: SimpleTextOutput,
    runtime_services: RuntimeServices,
    boot_services: BootServices,
    number_of_table_entries: usize,
    configuration_table: ConfigurationTable,
}

#[repr(C)]
struct ConfigurationTable {
    vendor_guid: Guid,
    vendor_table: *mut c_void,
}

#[repr(transparent)]
pub struct SystemTable {
    inner: *mut SystemTableRaw,
}

impl SystemTable {
    pub fn firmware_vendor(&self) -> &[u16] {
        unsafe { u16_slice_from_ptr((*self.inner).firmware_vendor) }
    }

    pub fn con_in(&self) -> &SimpleTextInput {
        unsafe { &(*self.inner).con_in }
    }

    pub fn con_out(&self) -> &SimpleTextOutput {
        unsafe { &(*self.inner).con_out }
    }
}
