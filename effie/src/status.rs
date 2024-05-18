use effie_macros::w;

pub type Result<T = (), E = Status> = core::result::Result<T, E>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Status(usize);

impl Status {
    pub const SUCCESS: Self = Self(0);

    pub const LOAD_ERROR: Self = Self::error(1);
    pub const INVALID_PARAMETER: Self = Self::error(2);
    pub const UNSUPPORTED: Self = Self::error(3);
    pub const BAD_BUFFER_SIZE: Self = Self::error(4);
    pub const BUFFER_TOO_SMALL: Self = Self::error(5);
    pub const NOT_READY: Self = Self::error(6);
    pub const DEVICE_ERROR: Self = Self::error(7);
    pub const WRITE_PROTECTED: Self = Self::error(8);
    pub const OUT_OF_RESOURCES: Self = Self::error(9);
    pub const VOLUME_CORRUPTED: Self = Self::error(10);
    pub const VOLUME_FULL: Self = Self::error(11);
    pub const NO_MEDIA: Self = Self::error(12);
    pub const MEDIA_CHANGED: Self = Self::error(13);
    pub const NOT_FOUND: Self = Self::error(14);
    pub const ACCESS_DENIED: Self = Self::error(15);
    pub const NO_RESPONSE: Self = Self::error(16);
    pub const NO_MAPPING: Self = Self::error(17);
    pub const TIMEOUT: Self = Self::error(18);
    pub const NOT_STARTED: Self = Self::error(19);
    pub const ALREADY_STARTED: Self = Self::error(20);
    pub const ABORTED: Self = Self::error(21);
    pub const ICMP_ERROR: Self = Self::error(22);
    pub const TFTP_ERROR: Self = Self::error(23);
    pub const PROTOCOL_ERROR: Self = Self::error(24);
    pub const INCOMPATIBLE_VERSION: Self = Self::error(25);
    pub const SECURITY_VIOLATION: Self = Self::error(26);
    pub const CRC_ERROR: Self = Self::error(27);
    pub const END_OF_MEDIA: Self = Self::error(28);
    pub const END_OF_FILE: Self = Self::error(31);
    pub const INVALID_LANGUAGE: Self = Self::error(32);
    pub const COMPROMISED_DATA: Self = Self::error(33);
    pub const IP_ADDRESS_CONFLICT: Self = Self::error(34);
    pub const HTTP_ERROR: Self = Self::error(35);

    pub const WARN_UNKNOWN_GLYPH: Self = Self(1);
    pub const WARN_DELETE_FAILURE: Self = Self(2);
    pub const WARN_WRITE_FAILURE: Self = Self(3);
    pub const WARN_BUFFER_TOO_SMALL: Self = Self(4);
    pub const WARN_STALE_DATA: Self = Self(5);
    pub const WARN_FILE_SYSTEM: Self = Self(6);
    pub const WARN_RESET_REQUIRED: Self = Self(7);

    const fn error(value: usize) -> Self {
        let mask: usize = 1 << (core::mem::size_of::<usize>() * 8 - 1);
        Self(value | mask)
    }

    pub const fn as_result(self) -> Result {
        match self {
            Self::SUCCESS => Ok(()),
            status => Err(status),
        }
    }

    pub fn as_result_with<T>(self, other: T) -> Result<T> {
        match self {
            Self::SUCCESS => Ok(other),
            status => Err(status),
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::SUCCESS)
    }

    pub fn is_error(self) -> bool {
        todo!()
    }

    pub fn is_warning(self) -> bool {
        todo!()
    }

    pub const fn description(&self) -> &[u16] {
        match *self {
            Self::SUCCESS => w!("The operation completed successfully."),
            Self::LOAD_ERROR => w!("The image failed to load."),
            Self::INVALID_PARAMETER => w!("A parameter was incorrect."),
            Self::UNSUPPORTED => w!("The operation is not supported."),
            Self::BAD_BUFFER_SIZE => w!("The buffer was not the proper size for the request."),
            Self::BUFFER_TOO_SMALL => w!("The buffer is not large enough to hold the requested data."),
            Self::NOT_READY => w!("There is no data pending upon return."),
            Self::DEVICE_ERROR => w!("The physical device reported an error while attempting the operation."),
            Self::WRITE_PROTECTED => w!("The device cannot be written to."),
            Self::OUT_OF_RESOURCES => w!("A resource has run out."),
            Self::VOLUME_CORRUPTED => w!("An inconstancy was detected on the file system causing the operating to fail."),
            Self::VOLUME_FULL => w!("There is no more space on the file system."),
            Self::NO_MEDIA => w!("The device does not contain any medium to perform the operation."),
            Self::MEDIA_CHANGED => w!("The medium in the device has changed since the last access."),
            Self::NOT_FOUND => w!("The item was not found."),
            Self::ACCESS_DENIED => w!("Access was denied."),
            Self::NO_RESPONSE => w!("The server was not found or did not respond to the request."),
            Self::NO_MAPPING => w!("A mapping to a device does not exist."),
            Self::TIMEOUT => w!("The timeout time expired."),
            Self::NOT_STARTED => w!("The protocol has not been started."),
            Self::ALREADY_STARTED => w!("The protocol has already been started."),
            Self::ABORTED => w!("The operation was aborted."),
            Self::ICMP_ERROR => w!("An ICMP error occurred during the network operation."),
            Self::TFTP_ERROR => w!("A TFTP error occurred during the network operation."),
            Self::PROTOCOL_ERROR => w!("A protocol error occurred during the network operation."),
            Self::INCOMPATIBLE_VERSION => w!("The function encountered an internal version that was incompatible with a version requested by the caller."),
            Self::SECURITY_VIOLATION => w!("The function was not performed due to a security violation."),
            Self::CRC_ERROR => w!("A CRC error was detected."),
            Self::END_OF_MEDIA => w!("Beginning or end of media was reached."),
            Self::END_OF_FILE => w!("The end of the file was reached."),
            Self::INVALID_LANGUAGE => w!("The language specified was invalid."),
            Self::COMPROMISED_DATA => w!("The security status of the data is unknown or compromised and the data must be updated or replaced to restore a valid security status."),
            Self::IP_ADDRESS_CONFLICT => w!("There is an address conflict address allocation"),
            Self::HTTP_ERROR => w!("A HTTP error occurred during the network operation."),
            Self::WARN_UNKNOWN_GLYPH => w!("The string contained one or more characters that the device could not render and were skipped."),
            Self::WARN_DELETE_FAILURE => w!("The handle was closed, but the file was not deleted."),
            Self::WARN_WRITE_FAILURE => w!("The handle was closed, but the data to the file was not flushed properly."),
            // Self::BUFFER_TOO_SMALL => w!("The resulting buffer was too small, and the data was truncated to the buffer size."),
            Self::WARN_STALE_DATA => w!("The data has not been updated within the timeframe set by local policy for this type of data."),
            Self::WARN_FILE_SYSTEM => w!("The resulting buffer contains UEFI-compliant file system."),
            Self::WARN_RESET_REQUIRED => w!("The operation will be processed across a system reset."),
            _ => w!("Unknown status.")
        }
    }
}
