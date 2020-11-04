use crate::stdio_lockers::{StdinLocker, StdoutLocker};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::{
    fmt::Arguments,
    fs::File,
    io::{self, IoSlice, IoSliceMut, Read, Write},
    mem::ManuallyDrop,
};

pub(crate) struct StdinRaw {
    file: ManuallyDrop<File>,
    _locker: StdinLocker,
}

pub(crate) struct StdoutRaw {
    file: ManuallyDrop<File>,
    _locker: StdoutLocker,
}

impl StdinRaw {
    pub(crate) fn new() -> Option<Self> {
        let locker = StdinLocker::new()?;

        #[cfg(not(windows))]
        let owned = unsafe { File::from_raw_fd(locker.as_raw_fd()) };
        #[cfg(windows)]
        let owned = unsafe { File::from_raw_handle(locker.as_raw_handle()) };

        let file = ManuallyDrop::new(owned);
        Some(Self {
            file,
            _locker: locker,
        })
    }
}

impl StdoutRaw {
    pub(crate) fn new() -> Option<Self> {
        let locker = StdoutLocker::new()?;

        #[cfg(not(windows))]
        let owned = unsafe { File::from_raw_fd(locker.as_raw_fd()) };
        #[cfg(windows)]
        let owned = unsafe { File::from_raw_handle(locker.as_raw_handle()) };

        let file = ManuallyDrop::new(owned);
        Some(Self {
            file,
            _locker: locker,
        })
    }
}

impl Read for StdinRaw {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.file.read_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.file.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.file.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.file.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.file.read_exact(buf)
    }
}

impl Write for StdoutRaw {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.file.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.file.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.file.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.file.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.file.write_fmt(fmt)
    }
}
