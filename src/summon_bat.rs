use crate::Type;
use raw_stdio::RawStdout;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::{
    fs::File,
    io,
    mem::ManuallyDrop,
    process::{Child, Command, Stdio},
};
use terminal_support::{detect_terminal_color_support, TerminalColorSupport};

/// Arrange for stdout to be connected to a pipe to a process which runs
/// bat to do syntax highlighting and paging.
///
/// TODO: Differentiate between classic ANSI 8 colors, 16 colors, the
/// 256-color cube, and possibly the 24-bit "true color".
pub(crate) fn summon_bat(
    stdout: &RawStdout,
    type_: &Type,
) -> io::Result<(bool, TerminalColorSupport, Option<Child>)> {
    let (isatty, color_support) = detect_terminal_color_support(&ManuallyDrop::new(unsafe {
        #[cfg(not(windows))]
        {
            File::from_raw_fd(stdout.as_raw_fd())
        }
        #[cfg(windows)]
        {
            File::from_raw_handle(stdout.as_raw_handle())
        }
    }));
    if !isatty {
        return Ok((isatty, color_support, None));
    }

    // If the "bat" command is available, use it.
    let child = match Command::new("bat")
        .arg("--file-name")
        .arg(type_.extension())
        .arg("--style")
        .arg("plain")
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok((isatty, color_support, None)),
    };

    Ok((isatty, color_support, Some(child)))
}
