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
}
