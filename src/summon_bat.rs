use crate::Type;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::{
    io,
    process::{Child, Command, Stdio},
};
use terminal_support::WriteTerminal;
use unsafe_io::AsUnsafeHandle;

/// Arrange for stdout to be connected to a pipe to a process which runs
/// bat to do syntax highlighting and paging.
pub(crate) fn summon_bat(
    stdout: &(impl WriteTerminal + AsUnsafeHandle),
    type_: &Type,
) -> io::Result<Option<Child>> {
    #[cfg(not(windows))]
    assert_eq!(stdout.as_unsafe_handle().as_raw_fd(), libc::STDOUT_FILENO);

    if !stdout.is_output_terminal() {
        return Ok(None);
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
        Err(_) => return Ok(None),
    };

    Ok(Some(child))
}
