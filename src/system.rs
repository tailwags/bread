use core::mem::MaybeUninit;
use r_efi::{
    efi::{self, Guid, Handle, Status},
    protocols::{
        file::Protocol as FileProtocol,
        loaded_image::{Protocol as LoadedImageProtocol, PROTOCOL_GUID as LOADED_IMAGE_GUID},
        simple_file_system::{
            Protocol as SimpleFileSystemProtocol, PROTOCOL_GUID as SIMPLE_FILE_SYSTEM_GUID,
        },
    },
};

#[repr(transparent)]
pub struct SystemTable {
    inner: *mut efi::SystemTable,
}

impl SystemTable {
    pub const unsafe fn from_raw(raw: *mut efi::SystemTable) -> Self {
        Self { inner: raw }
    }

    pub const fn as_raw(&self) -> *mut efi::SystemTable {
        self.inner
    }

    pub fn print(&self, string: *mut u16) -> Status {
        unsafe { ((*(*self.inner).con_out).output_string)((*self.inner).con_out, string) }
    }

    pub fn get_volume(&self, image_handle: Handle) -> *mut FileProtocol {
        unsafe {
            let loaded_image =
                self.handle_protocol::<LoadedImageProtocol>(image_handle, &LOADED_IMAGE_GUID);

            let filesystem = self.handle_protocol::<SimpleFileSystemProtocol>(
                loaded_image.cast(),
                &SIMPLE_FILE_SYSTEM_GUID,
            );

            let mut volume = MaybeUninit::<*mut FileProtocol>::uninit();

            ((*filesystem).open_volume)(filesystem, volume.as_mut_ptr());

            let volume = volume.assume_init();

            volume
        }
    }

    pub fn handle_protocol<P>(&self, handle: Handle, guid: *const Guid) -> *mut P {
        let boot_services = unsafe { (*self.inner).boot_services };

        let mut protocol = MaybeUninit::<*mut P>::uninit();

        let status = unsafe {
            ((*boot_services).handle_protocol)(handle, guid as *mut _, protocol.as_mut_ptr().cast())
        };

        unsafe { protocol.assume_init() }
    }
}
