//! Define `CommandStdinStdout`, a lazy form of `ChildStdinStdout` which
//! launches the child process on demand. This works better with
//! command-line parsers like `clap` which call `from_str` multiple times
//! and expect to be able to harmlessly discard the results.

use crate::child_stdin_stdout::ChildStdinStdout;
use std::process::{Command, Stdio};
use std::{
    fmt::Arguments,
    io::{self, IoSlice, IoSliceMut, Read, Write},
};

/// A child's (stdin, stdout) pair which can implement the `ReadWrite` trait.
pub(crate) struct CommandStdinStdout {
    command: Command,
    child: Option<ChildStdinStdout>,
}

impl CommandStdinStdout {
    pub(crate) fn new(mut command: Command) -> Self {
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        Self {
            command,
            child: None,
        }
    }

    fn child(&mut self) -> io::Result<&mut ChildStdinStdout> {
        if let Some(ref mut child) = self.child {
            Ok(child)
        } else {
            let child = self.command.spawn()?;
            self.child = ChildStdinStdout::new(child);
            Ok(self.child.as_mut().unwrap())
        }
    }
}

impl Read for CommandStdinStdout {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.child()?.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.child()?.read_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.child()?.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.child()?.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.child()?.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.child()?.read_exact(buf)
    }
}

impl Write for CommandStdinStdout {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.child()?.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.child()?.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.child()?.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.child()?.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.child()?.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.child()?.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.child()?.write_fmt(fmt)
    }
}
