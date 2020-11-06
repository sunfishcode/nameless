use std::io::{Read, Write};

/// A combined `Read` and `Write` trait, particularly for use with interactive
/// streams that support reading and writing.
pub trait ReadWrite: Read + Write {}

// Implement `ReadWrite` for our stream types.
impl ReadWrite for crate::stdin_stdout::StdinStdout {}
impl ReadWrite for crate::child_stdin_stdout::ChildStdinStdout {}
impl ReadWrite for crate::command_stdin_stdout::CommandStdinStdout {}
impl ReadWrite for crate::interactive_byte_stream::InteractiveByteStream {}
impl<RW: ReadWrite> ReadWrite for crate::BufReaderWriter<RW> {}

// Implement `ReadWrite` for `std`'s stream types. Note that we include
// `File` even though regular files aren't interactive, because device
// files may be interactive.
impl ReadWrite for std::net::TcpStream {}
impl ReadWrite for std::fs::File {}
#[cfg(not(windows))]
impl ReadWrite for std::os::unix::net::UnixStream {}

// Implement `ReadWrite` for the `readwrite` crate, if present.
#[cfg(feature = "readwrite")]
impl ReadWrite for readwrite::ReadWrite {}
