use core::ffi::c_void;

use crate::{
    protocols::{SimpleTextInput, SimpleTextOutput},
    tables::{BootServices, RuntimeServices, TableHeader},
    util::u16_slice_from_ptr,
    Guid, Handle,
};

use super::{Signature, SpecificationRevision};

/// Contains pointers to the runtime and boot services tables.
#[repr(C)]
pub struct SystemTable {
    /// TODO
    hdr: TableHeader,
    /// A pointer to a null terminated string that identifies the vendor that produces the system firmware for the platform
    firmware_vendor: *const u16,
    /// A firmware vendor specific value that identifies the revision of the system firmware for the platform.
    firmware_version: u32,
    /// The handle for the active console input device.
    console_in_handle: Handle,
    /// A pointer to the EFI_SIMPLE_TEXT_INPUT_PROTOCOL interface that is associated with ConsoleInHandle.
    con_in: *const SimpleTextInput,
    console_out_handle: Handle,
    con_out: *const SimpleTextOutput,
    standard_error_handler: Handle,
    std_err: *const SimpleTextOutput,
    runtime_services: *const RuntimeServices,
    pub(crate) boot_services: *const BootServices,
    number_of_table_entries: usize,
    configuration_table: ConfigurationTable,
}

#[repr(C)]
struct ConfigurationTable {
    vendor_guid: Guid,
    vendor_table: *mut c_void,
}

impl SystemTable {
    pub fn signature(&self) -> Signature {
        self.hdr.signature
    }

    pub fn revision(&self) -> SpecificationRevision {
        self.hdr.revision
    }

    pub fn firmware_vendor(&self) -> &[u16] {
        unsafe { u16_slice_from_ptr(self.firmware_vendor) }
    }

    pub fn con_in(&self) -> &SimpleTextInput {
        unsafe { &*self.con_in }
    }

    pub fn con_out(&self) -> &SimpleTextOutput {
        unsafe { &*self.con_out }
    }

    pub fn boot_services(&self) -> &BootServices {
        unsafe { &*self.boot_services }
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        unsafe { &*self.runtime_services }
    }
}
