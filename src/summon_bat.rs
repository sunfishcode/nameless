//! Wrap stdout in a [`bat`].
//!
//! [`bat`]: https://crates.io/crates/bat

use crate::MediaType;
use io_extras::grip::AsRawGrip;
use std::process::{Child, Command, Stdio};

/// Arrange for stdout to be connected to a pipe to a process which runs
/// bat to do syntax highlighting and paging.
pub(crate) fn summon_bat(stdout: &impl AsRawGrip, media_type: &MediaType) -> Option<Child> {
    assert_eq!(stdout.as_raw_grip(), std::io::stdout().as_raw_grip());

    // If the "bat" command is available, use it.
    Command::new("bat")
        .arg("--file-name")
        .arg(media_type.extension())
        .arg("--style")
        .arg("plain")
        .stdin(Stdio::piped())
        .spawn()
        .ok()
}
