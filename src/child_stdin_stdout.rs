//! Define `ChildStdinStdout`, an interactive stream object formed by
//! combining a child process' stdin and stdout.

use std::process::{Child, ChildStdin, ChildStdout};
use std::{
    fmt::Arguments,
    io::{self, IoSlice, IoSliceMut, Read, Write},
};

/// A child's (stdin, stdout) pair which can implement the `ReadWrite` trait.
pub(crate) struct ChildStdinStdout {
    _child: Child, // fixme?
    stdout: ChildStdout,
    stdin: ChildStdin,
}

impl ChildStdinStdout {
    pub(crate) fn new(mut child: Child) -> Option<Self> {
        let stdout = child.stdout.take()?;
        let stdin = child.stdin.take()?;
        Some(Self { _child: child, stdin, stdout })
    }
}

impl Read for ChildStdinStdout {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.stdout.read_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.stdout.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.stdout.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.stdout.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.stdout.read_exact(buf)
    }
}

impl Write for ChildStdinStdout {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdin.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.stdin.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.stdin.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.stdin.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stdin.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.stdin.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.stdin.write_fmt(fmt)
    }
}
