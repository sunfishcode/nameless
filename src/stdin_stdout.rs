//! Define `StdinStdout`, an interactive stream object formed by combining
//! stdin and stdout.

use io_ext::{ReadExt, Status, WriteExt};
use raw_stdio::{RawStdin, RawStdout};
use std::{
    fmt::Arguments,
    io::{self, IoSlice, IoSliceMut},
};

/// A raw (stdin, stdout) pair which can implement the `ReadWrite` trait.
pub(crate) struct StdinStdout {
    stdin: RawStdin,
    stdout: RawStdout,
}

impl StdinStdout {
    pub(crate) fn new() -> Option<Self> {
        Some(Self {
            stdin: RawStdin::new()?,
            stdout: RawStdout::new()?,
        })
    }
}

impl ReadExt for StdinStdout {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        self.stdin.read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        self.stdin.read_vectored_with_status(bufs)
    }
}

impl io::Read for StdinStdout {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdin.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.stdin.read_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.stdin.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.stdin.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.stdin.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.stdin.read_exact(buf)
    }
}

impl WriteExt for StdinStdout {
    #[inline]
    fn flush_with_status(&mut self, status: Status) -> io::Result<()> {
        self.stdout.flush_with_status(status)
    }

    #[inline]
    fn abandon(&mut self) {
        self.stdout.abandon()
    }

    #[inline]
    fn write_str(&mut self, buf: &str) -> io::Result<()> {
        self.stdout.write_str(buf)
    }
}

impl io::Write for StdinStdout {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.stdout.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.stdout.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stdout.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.stdout.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.stdout.write_fmt(fmt)
    }
}
