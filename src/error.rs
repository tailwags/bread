//! Bread's error type: a message saying what failed, plus the firmware status
//! when there is one.
//!
//! Everything that can fail before the kernel handoff returns [`Result`].
//! Errors carry no "recoverable" flag: whoever catches one decides what
//! happens next (see `main`), and shows it on an error screen, which also
//! saves the log. A message may contain `\n` to break lines on that screen.

use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt;

pub type Result<T = ()> = core::result::Result<T, Error>;

pub struct Error {
    message: Message,
    status: Option<effie::Status>,
}

impl Error {
    pub fn new(message: impl Into<Message>) -> Self {
        Self {
            message: message.into(),
            status: None,
        }
    }
}

/// An error message. Messages without format arguments are kept as
/// `&'static str`, so building them neither formats nor allocates.
pub struct Message(Cow<'static, str>);

impl From<&'static str> for Message {
    fn from(message: &'static str) -> Self {
        Self(Cow::Borrowed(message))
    }
}

impl From<String> for Message {
    fn from(message: String) -> Self {
        Self(Cow::Owned(message))
    }
}

impl From<fmt::Arguments<'_>> for Message {
    fn from(args: fmt::Arguments<'_>) -> Self {
        match args.as_str() {
            Some(message) => message.into(),
            None => alloc::fmt::format(args).into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            Some(status) => write!(f, "{}: {status}", self.message.0),
            None => f.write_str(&self.message.0),
        }
    }
}

/// Attaches a message to a failed firmware call.
pub trait Context<T> {
    fn context(self, message: impl Into<Message>) -> Result<T>;
}

impl<T> Context<T> for effie::Result<T> {
    fn context(self, message: impl Into<Message>) -> Result<T> {
        self.map_err(|status| Error {
            message: message.into(),
            status: Some(status),
        })
    }
}

/// Returns an [`Error`] built from a format string unless `cond` holds.
macro_rules! ensure {
    ($cond:expr, $($msg:tt)+) => {
        if !$cond {
            return Err($crate::error::Error::new(format_args!($($msg)+)));
        }
    };
}
pub(crate) use ensure;
