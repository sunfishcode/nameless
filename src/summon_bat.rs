use crate::Type;
use std::{
    fs::File,
    io,
    mem::ManuallyDrop,
    os::unix::io::{AsRawFd, FromRawFd},
    process::{Child, Command, Stdio},
};
use terminal_color_support::{detect_terminal_color_support, TerminalColorSupport};

/// Arrange for stdout to be connected to a pipe to a process which runs
/// bat to do syntax highlighting and paging.
///
/// TODO: Differentiate between classic ANSI 8 colors, 16 colors, the
/// 256-color cube, and possibly the 24-bit "true color".
pub(crate) fn summon_bat(type_: &Type) -> io::Result<(bool, TerminalColorSupport, Option<Child>)> {
    let (isatty, color_support) = detect_terminal_color_support(&ManuallyDrop::new(unsafe {
        File::from_raw_fd(libc::STDOUT_FILENO)
    }));
    if !isatty {
        return Ok((isatty, color_support, None));
    }

    // If the "bat" command is available, use it.
    let mut child = match Command::new("bat")
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

    // Redirect our stdout to the child's stdin.
    if unsafe {
        libc::dup3(
            child.stdin.as_ref().unwrap().as_raw_fd(),
            libc::STDOUT_FILENO,
            libc::O_CLOEXEC,
        )
    } == -1
    {
        child.wait()?;
        return Err(io::Error::last_os_error());
    }

    Ok((isatty, color_support, Some(child)))
}
