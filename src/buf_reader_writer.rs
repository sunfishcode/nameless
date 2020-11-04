//! This file is derived from Rust's library/std/src/io/buffered at revision
//! a78a62fc996ba16f7a111c99520b23f77029f4eb.

#![allow(missing_docs)] // TODO: Link to the corresponding `std` docs.

use crate::ReadWrite;
use std::{
    cmp, fmt,
    io::{self, BufRead, Error, ErrorKind, IoSlice, IoSliceMut, Read, Write},
};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// A combined `BufReader` and `BufWriter` for types that implement `ReadWrite`,
/// which is a combined `Read` and `Write` trait.
pub struct BufReaderWriter<RW: ReadWrite> {
    inner: RW,

    // reader state
    reader_buf: Box<[u8]>,
    pos: usize,
    cap: usize,

    // writer state
    writer_buf: Vec<u8>,
    // #30888: If the inner writer panics in a call to write, we don't want to
    // write the buffered data a second time in BufReaderWriter's destructor. This
    // flag tells the Drop impl if it should skip the flush.
    panicked: bool,
}

// reader methods
impl<RW: ReadWrite> BufReaderWriter<RW> {
    pub fn new(inner: RW) -> Self {
        Self::with_capacities(DEFAULT_BUF_SIZE, DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacities(reader_capacity: usize, writer_capacity: usize, inner: RW) -> Self {
        #[cfg(not(feature = "nightly"))]
        let buffer = vec![0; reader_capacity];
        #[cfg(feature = "nightly")]
        let buffer = unsafe {
            let mut buffer = Vec::with_capacity(reader_capacity);
            buffer.set_len(reader_capacity);
            inner.initializer().initialize(&mut buffer);
            buffer
        };
        Self {
            inner,
            reader_buf: buffer.into_boxed_slice(),
            pos: 0,
            cap: 0,
            writer_buf: Vec::with_capacity(writer_capacity),
            panicked: false,
        }
    }

    pub fn get_ref(&self) -> &RW {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut RW {
        &mut self.inner
    }

    pub fn reader_buffer(&self) -> &[u8] {
        &self.reader_buf[self.pos..self.cap]
    }

    pub fn reader_capacity(&self) -> usize {
        self.reader_buf.len()
    }

    #[inline]
    fn discard_reader_buffer(&mut self) {
        self.pos = 0;
        self.cap = 0;
    }
}

// writer methods
impl<RW: ReadWrite> BufReaderWriter<RW> {
    pub(super) fn flush_buf(&mut self) -> io::Result<()> {
        struct BufGuard<'a> {
            buffer: &'a mut Vec<u8>,
            written: usize,
        }

        impl<'a> BufGuard<'a> {
            fn new(buffer: &'a mut Vec<u8>) -> Self {
                Self { buffer, written: 0 }
            }

            fn remaining(&self) -> &[u8] {
                &self.buffer[self.written..]
            }

            fn consume(&mut self, amt: usize) {
                self.written += amt;
            }

            fn done(&self) -> bool {
                self.written >= self.buffer.len()
            }
        }

        impl Drop for BufGuard<'_> {
            fn drop(&mut self) {
                if self.written > 0 {
                    self.buffer.drain(..self.written);
                }
            }
        }

        let mut guard = BufGuard::new(&mut self.writer_buf);
        let inner = &mut self.inner;
        while !guard.done() {
            self.panicked = true;
            let r = inner.write(guard.remaining());
            self.panicked = false;

            match r {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write the buffered data",
                    ));
                }
                Ok(n) => guard.consume(n),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub(super) fn write_to_buf(&mut self, buf: &[u8]) -> usize {
        let available = self.writer_buf.capacity() - self.writer_buf.len();
        let amt_to_buffer = available.min(buf.len());
        self.writer_buf.extend_from_slice(&buf[..amt_to_buffer]);
        amt_to_buffer
    }

    pub fn writer_buffer(&self) -> &[u8] {
        &self.writer_buf
    }

    pub fn writer_capacity(&self) -> usize {
        self.writer_buf.capacity()
    }

    // FIXME: IntoInnerError doesn't expose its new function.
    /*
    pub fn into_inner(mut self) -> Result<RW, IntoInnerError<Self>> {
        match self.flush_buf() {
            Err(e) => Err(IntoInnerError::new(self, e)),
            Ok(()) => Ok(self.inner),
        }
    }
    */
}

impl<RW: ReadWrite> Read for BufReaderWriter<RW> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Flush the writer half of this `BufReaderWriter` before reading.
        self.flush()?;

        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.pos == self.cap && buf.len() >= self.reader_buf.len() {
            self.discard_reader_buffer();
            return self.inner.read(buf);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        // Flush the writer half of this `BufReaderWriter` before reading.
        self.flush()?;

        let total_len = bufs.iter().map(|b| b.len()).sum::<usize>();
        if self.pos == self.cap && total_len >= self.reader_buf.len() {
            self.discard_reader_buffer();
            return self.inner.read_vectored(bufs);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read_vectored(bufs)?
        };
        self.consume(nread);
        Ok(nread)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.inner.is_read_vectored()
    }
}

impl<RW: ReadWrite> BufRead for BufReaderWriter<RW> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // If we've reached the end of our internal buffer then we need to fetch
        // some more data from the underlying reader.
        // Branch using `>=` instead of the more correct `==`
        // to tell the compiler that the pos..cap slice is always valid.
        if self.pos >= self.cap {
            // Flush the writer half of this `BufReaderWriter` before reading.
            self.flush()?;

            debug_assert_eq!(self.pos, self.cap);
            self.cap = self.inner.read(&mut self.reader_buf)?;
            self.pos = 0;
        }
        Ok(&self.reader_buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.cap);
    }
}

impl<RW: ReadWrite> Write for BufReaderWriter<RW> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.writer_buf.len() + buf.len() > self.writer_buf.capacity() {
            self.flush_buf()?;
        }
        // FIXME: Why no len > capacity? Why not buffer len == capacity? #72919
        if buf.len() >= self.writer_buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write(buf);
            self.panicked = false;
            r
        } else {
            self.writer_buf.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        // Normally, `write_all` just calls `write` in a loop. We can do better
        // by calling `self.get_mut().write_all()` directly, which avoids
        // round trips through the buffer in the event of a series of partial
        // writes in some circumstances.
        if self.writer_buf.len() + buf.len() > self.writer_buf.capacity() {
            self.flush_buf()?;
        }
        // FIXME: Why no len > capacity? Why not buffer len == capacity? #72919
        if buf.len() >= self.writer_buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write_all(buf);
            self.panicked = false;
            r
        } else {
            self.writer_buf.extend_from_slice(buf);
            Ok(())
        }
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let total_len = bufs.iter().map(|b| b.len()).sum::<usize>();
        if self.writer_buf.len() + total_len > self.writer_buf.capacity() {
            self.flush_buf()?;
        }
        // FIXME: Why no len > capacity? Why not buffer len == capacity? #72919
        if total_len >= self.writer_buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write_vectored(bufs);
            self.panicked = false;
            r
        } else {
            bufs.iter()
                .for_each(|b| self.writer_buf.extend_from_slice(b));
            Ok(total_len)
        }
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.get_ref().is_write_vectored()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf().and_then(|()| self.get_mut().flush())
    }
}

impl<RW: ReadWrite> Drop for BufReaderWriter<RW> {
    fn drop(&mut self) {
        if !self.panicked {
            // dtors should not panic, so we ignore a failed flush
            let _r = self.flush_buf();
        }
    }
}

impl<RW: ReadWrite> fmt::Debug for BufReaderWriter<RW>
where
    RW: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BufReaderWriter")
            .field("inner", &self.inner)
            .field(
                "reader_buffer",
                &format_args!("{}/{}", self.cap - self.pos, self.reader_buf.len()),
            )
            .field(
                "writer_buffer",
                &format_args!("{}/{}", self.writer_buf.len(), self.writer_buf.capacity()),
            )
            .finish()
    }
}
