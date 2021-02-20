//! Temporary copy of `TryFromOsArg` until it becomes clear whether we'll
//! submit such changes upstream.

use std::ffi::{OsStr, OsString};

/// Parse a value from an `OsString`, possibly with side effects.
///
/// This is intended to be used with `OsString` values returned from
/// [`ArgsOs`] as part of command-line parsing which typically happens
/// at most once per process. Unlike `FromStr`, `From`, or `TryFrom`, it
/// may have side effects such as opening files or network connections.
///
/// [`ArgsOs`]: std::env::ArgsOs
pub trait TryFromOsArg: Sized {
    /// The associated error which can be returned from parsing.
    type Error;

    /// Parses an OS string `os` to return a value of this type, with side
    /// effects appropriate to converting command-line strings into resource
    /// handles.
    ///
    /// If parsing succeeds, return the value inside `Ok`, otherwise when the
    /// string is ill-formatted return an error specific to the inside `Err`.
    /// The error type is specific to implementation of the trait.
    fn try_from_os_str_arg(os: &OsStr) -> Result<Self, Self::Error>;

    /// Like `try_from_os_str_arg`, but takes an `OsString` instead. Types
    /// should manually implement this if they can reuse an existing `OsString`
    /// allocation.
    #[inline]
    fn try_from_os_string_arg(os: OsString) -> Result<Self, Self::Error> {
        Self::try_from_os_str_arg(&os)
    }
}
