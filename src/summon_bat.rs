use crate::Type;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use unsafe_io::AsUnsafeHandle;

/// Arrange for stdout to be connected to a pipe to a process which runs
/// bat to do syntax highlighting and paging.
pub(crate) fn summon_bat(stdout: &impl AsUnsafeHandle, type_: &Type) -> Option<Child> {
    #[cfg(not(windows))]
    assert_eq!(stdout.as_unsafe_handle().as_raw_fd(), libc::STDOUT_FILENO);

    // If the "bat" command is available, use it.
    Command::new("bat")
        .arg("--file-name")
        .arg(type_.extension())
        .arg("--style")
        .arg("plain")
        .stdin(Stdio::piped())
        .spawn()
        .ok()
}
