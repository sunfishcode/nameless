//! This file is derived from Rust's library/std/src/io/buffered at revision
//! a78a62fc996ba16f7a111c99520b23f77029f4eb.

#![allow(missing_docs)] // TODO: Link to the corresponding `std` docs.

use crate::{buf_reader_line_writer_shim::BufReaderLineWriterShim, BufReaderWriter, ReadWrite};
use std::{
    fmt,
    io::{self, BufRead, IoSlice, IoSliceMut, Read, Write},
};

/// A combined `BufReader` and `LineWriter` for types that implement
/// `ReadWrite`, which is a combined `Read` and `Write` trait.
pub struct BufReaderLineWriter<RW: ReadWrite> {
    inner: BufReaderWriter<RW>,
}

impl<RW: ReadWrite> BufReaderLineWriter<RW> {
    pub fn new(inner: RW) -> Self {
        // Lines typically aren't that long, don't use a giant buffer
        Self::with_capacities(1024, 1024, inner)
    }

    pub fn with_capacities(reader_capacity: usize, writer_capacity: usize, inner: RW) -> Self {
        Self {
            inner: BufReaderWriter::with_capacities(reader_capacity, writer_capacity, inner),
        }
    }

    pub fn get_ref(&self) -> &RW {
        self.inner.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut RW {
        self.inner.get_mut()
    }

    // FIXME: IntoInnerError doesn't expose its new function.
    /*
    pub fn into_inner(self) -> Result<RW, IntoInnerError<Self>> {
        self.inner.into_inner().map_err(|err| err.new_wrapped(|inner| LineWriter { inner }))
    }
    */
}

// reader methods
impl<RW: ReadWrite> BufReaderLineWriter<RW> {
    #[inline]
    pub fn reader_buffer(&self) -> &[u8] {
        self.inner.reader_buffer()
    }

    #[inline]
    pub fn reader_capacity(&self) -> usize {
        self.inner.reader_capacity()
    }
}

impl<RW: ReadWrite> Read for BufReaderLineWriter<RW> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.inner.is_read_vectored()
    }
}

impl<RW: ReadWrite> BufRead for BufReaderLineWriter<RW> {
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl<RW: ReadWrite> Write for BufReaderLineWriter<RW> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        BufReaderLineWriterShim::new(&mut self.inner).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        BufReaderLineWriterShim::new(&mut self.inner).write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        BufReaderLineWriterShim::new(&mut self.inner).write_all(buf)
    }

    #[cfg(feature = "nightly")]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        BufReaderLineWriterShim::new(&mut self.inner).write_all_vectored(bufs)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        BufReaderLineWriterShim::new(&mut self.inner).write_fmt(fmt)
    }
}

impl<RW: ReadWrite> fmt::Debug for BufReaderLineWriter<RW>
where
    RW: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("LineWriter")
            .field("inner", &self.get_ref())
            .field(
                "reader_buffer",
                &format_args!(
                    "{}/{}",
                    self.inner.reader_buffer().len(),
                    self.inner.reader_capacity()
                ),
            )
            .field(
                "writer_buffer",
                &format_args!(
                    "{}/{}",
                    self.inner.writer_buffer().len(),
                    self.inner.writer_capacity()
                ),
            )
            .finish()
    }
}
