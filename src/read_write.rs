use io_ext::{ReadExt, ReadWriteExt, WriteExt};
use std::io;

// Implement `ReadWriteExt` for our stream types.
impl ReadWriteExt for crate::stdin_stdout::StdinStdout {}
#[cfg(not(windows))]
impl ReadWriteExt for crate::child_stdin_stdout::ChildStdinStdout {}
#[cfg(not(windows))]
impl ReadWriteExt for crate::command_stdin_stdout::CommandStdinStdout {}
impl ReadWriteExt for crate::interactive_byte_stream::InteractiveByteStream {}
impl<RW: io::Read + io::Write + ReadExt + WriteExt> ReadWriteExt for crate::BufReaderWriter<RW> {}
