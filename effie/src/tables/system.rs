use core::{ffi::c_void, ptr::NonNull};

use crate::{
    protocols::{SimpleTextInput, SimpleTextOutput},
    tables::{BootServices, RuntimeServices, TableHeader},
    u16_slice_from_ptr, Guid, Handle,
};

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

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SystemTable {
    inner: NonNull<SystemTableRaw>,
}

impl SystemTable {
    pub(crate) const unsafe fn as_raw(&self) -> *mut SystemTableRaw {
        self.inner.as_ptr()
    }

    pub(crate) const unsafe fn from_raw(raw: *mut SystemTableRaw) -> Self {
        Self {
            inner: NonNull::new_unchecked(raw),
        }
    }

    pub fn firmware_vendor(&self) -> &[u16] {
        unsafe { u16_slice_from_ptr((*self.as_raw()).firmware_vendor) }
    }

    pub fn con_in(&self) -> &SimpleTextInput {
        unsafe { &(*self.as_raw()).con_in }
    }

    pub fn con_out(&self) -> &SimpleTextOutput {
        unsafe { &(*self.as_raw()).con_out }
    }

    pub fn boot_services(&self) -> &BootServices {
        unsafe { &(*self.as_raw()).boot_services }
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        unsafe { &(*self.as_raw()).runtime_services }
    }
}
