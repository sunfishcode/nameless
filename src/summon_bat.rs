//! Wrap stdout in a [`bat`].
//!
//! [`bat`]: https://crates.io/crates/bat

use crate::Type;
use std::process::{Child, Command, Stdio};
use unsafe_io::AsUnsafeHandle;

/// Arrange for stdout to be connected to a pipe to a process which runs
/// bat to do syntax highlighting and paging.
pub(crate) fn summon_bat(stdout: &impl AsUnsafeHandle, type_: &Type) -> Option<Child> {
    assert!(unsafe {
        stdout
            .as_unsafe_handle()
            .as_unsafe_handle()
            .eq(std::io::stdout().as_unsafe_handle())
    });

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
