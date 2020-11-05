//! This file is derived from Rust's library/std/src/io/buffered at revision
//! f7801d6c7cc19ab22bdebcc8efa894a564c53469.

use std::cmp;
use std::fmt;
use std::io::{
    self, BufRead, Error, ErrorKind, IoSlice, IoSliceMut, Read, Write,
};
#[cfg(feature = "nightly")]
use std::io::Initializer;
use super::{DEFAULT_BUF_SIZE, IntoInnerError};
use crate::ReadWrite;

/// Wraps a reader and writer and buffers their output.
///
/// It can be excessively inefficient to work directly with something that
/// implements [`Write`]. For example, every call to
/// [`write`][`TcpStream::write`] on [`TcpStream`] results in a system call. A
/// `BufReaderWriter<RW>` keeps an in-memory buffer of data and writes it to an underlying
/// writer in large, infrequent batches.
///
/// It can be excessively inefficient to work directly with a [`Read`] instance.
/// For example, every call to [`read`][`TcpStream::read`] on [`TcpStream`]
/// results in a system call. A `BufReaderWriter<RW>` performs large, infrequent reads on
/// the underlying [`Read`] and maintains an in-memory buffer of the results.
///
/// `BufReaderWriter<RW>` can improve the speed of programs that make *small* and
/// *repeated* write calls to the same file or network socket. It does not
/// help when writing very large amounts at once, or writing just one or a few
/// times. It also provides no advantage when writing to a destination that is
/// in memory, like a [`Vec`]`<u8>`.
///
/// `BufReaderWriter<RW>` can improve the speed of programs that make *small* and
/// *repeated* read calls to the same file or network socket. It does not
/// help when reading very large amounts at once, or reading just one or a few
/// times. It also provides no advantage when reading from a source that is
/// already in memory, like a [`Vec`]`<u8>`.
///
/// It is critical to call [`flush`] before `BufReaderWriter<RW>` is dropped. Though
/// dropping will attempt to flush the contents of the writer buffer, any errors
/// that happen in the process of dropping will be ignored. Calling [`flush`]
/// ensures that the writer buffer is empty and thus dropping will not even attempt
/// file operations.
///
/// When the `BufReaderWriter<RW>` is dropped, the contents of its reader buffer will be
/// discarded. Creating multiple instances of a `BufReaderWriter<RW>` on the same
/// stream can cause data loss. Reading from the underlying reader after
/// unwrapping the `BufReaderWriter<RW>` with [`BufReaderWriter::into_inner`] can also cause
/// data loss.
///
/// # Examples
///
/// Let's write the numbers one through ten to a [`TcpStream`]:
///
/// ```no_run
/// use std::io::prelude::*;
/// use std::net::TcpStream;
///
/// let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();
///
/// for i in 0..10 {
///     stream.write(&[i+1]).unwrap();
/// }
/// ```
///
/// Because we're not buffering, we write each one in turn, incurring the
/// overhead of a system call per byte written. We can fix this with a
/// `BufReaderWriter<RW>`:
///
/// ```no_run
/// use std::io::prelude::*;
/// use std::net::TcpStream;
/// use nameless::BufReaderWriter;
///
/// let mut stream = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
///
/// for i in 0..10 {
///     stream.write(&[i+1]).unwrap();
/// }
/// stream.flush().unwrap();
/// ```
///
/// By wrapping the stream with a `BufReaderWriter<RW>`, these ten writes are all grouped
/// together by the buffer and will all be written out in one system call when
/// the `stream` is flushed.
///
/// ```no_run
/// use std::io::prelude::*;
/// use std::net::TcpStream;
/// use nameless::BufReaderWriter;
///
/// fn main() -> std::io::Result<()> {
///     let mut stream = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
///
///     let mut line = String::new();
///     let len = stream.read_line(&mut line)?;
///     println!("First line is {} bytes long", len);
///     Ok(())
/// }
/// ```
///
/// [`TcpStream::read`]: std::io::Read::read
/// [`TcpStream::write`]: std::io::Write::write
/// [`TcpStream`]: std::net::TcpStream
/// [`flush`]: std::io::Write::flush
pub struct BufReaderWriter<RW: ReadWrite> {
    inner: Option<RW>,
    writer_buf: Vec<u8>,
    // #30888: If the inner writer panics in a call to write, we don't want to
    // write the buffered data a second time in BufReaderWriter's destructor. This
    // flag tells the Drop impl if it should skip the flush.
    panicked: bool,

    // reader fields
    reader_buf: Box<[u8]>,
    pos: usize,
    cap: usize,
}

impl<RW: ReadWrite> BufReaderWriter<RW> {
    /// Creates a new `BufReaderWriter<RW>` with default buffer capacities. The default is currently 8 KB,
    /// but may change in the future.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::TcpStream;
    /// use nameless::BufReaderWriter;
    ///
    /// let mut buffer = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    /// ```
    pub fn new(inner: RW) -> Self {
        Self::with_capacities(DEFAULT_BUF_SIZE, DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufReaderWriter<RW>` with the specified buffer capacities.
    ///
    /// # Examples
    ///
    /// Creating a buffer with ten bytes of reader capacity and a writer buffer of a hundered bytes:
    ///
    /// ```no_run
    /// use std::net::TcpStream;
    /// use nameless::BufReaderWriter;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:34254").unwrap();
    /// let mut buffer = BufReaderWriter::with_capacities(10, 100, stream);
    /// ```
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
            inner: Some(inner),
            writer_buf: Vec::with_capacity(writer_capacity),
            panicked: false,
            reader_buf: buffer.into_boxed_slice(),
            pos: 0,
            cap: 0,
        }
    }

    /// Send data in our local buffer into the inner writer, looping as
    /// necessary until either it's all been sent or an error occurs.
    ///
    /// Because all the data in the buffer has been reported to our owner as
    /// "successfully written" (by returning nonzero success values from
    /// `write`), any 0-length writes from `inner` must be reported as i/o
    /// errors from this method.
    pub(super) fn flush_buf(&mut self) -> io::Result<()> {
        /// Helper struct to ensure the buffer is updated after all the writes
        /// are complete. It tracks the number of written bytes and drains them
        /// all from the front of the buffer when dropped.
        struct BufGuard<'a> {
            buffer: &'a mut Vec<u8>,
            written: usize,
        }

        impl<'a> BufGuard<'a> {
            fn new(buffer: &'a mut Vec<u8>) -> Self {
                Self { buffer, written: 0 }
            }

            /// The unwritten part of the buffer
            fn remaining(&self) -> &[u8] {
                &self.buffer[self.written..]
            }

            /// Flag some bytes as removed from the front of the buffer
            fn consume(&mut self, amt: usize) {
                self.written += amt;
            }

            /// true if all of the bytes have been written
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
        let inner = self.inner.as_mut().unwrap();
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

    /// Buffer some data without flushing it, regardless of the size of the
    /// data. Writes as much as possible without exceeding capacity. Returns
    /// the number of bytes written.
    pub(super) fn write_to_buf(&mut self, buf: &[u8]) -> usize {
        let available = self.writer_buf.capacity() - self.writer_buf.len();
        let amt_to_buffer = available.min(buf.len());
        self.writer_buf.extend_from_slice(&buf[..amt_to_buffer]);
        amt_to_buffer
    }

    /// Gets a reference to the underlying reader/writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nameless::BufReaderWriter;
    /// use std::net::TcpStream;
    ///
    /// let mut buffer = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // we can use reference just like buffer
    /// let reference = buffer.get_ref();
    /// ```
    #[inline]
    pub fn get_ref(&self) -> &RW {
        self.inner.as_ref().unwrap()
    }

    /// Gets a mutable reference to the underlying reader/writer.
    ///
    /// It is inadvisable to directly write to the underlying reader/writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nameless::BufReaderWriter;
    /// use std::net::TcpStream;
    ///
    /// let mut buffer = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // we can use reference just like buffer
    /// let reference = buffer.get_mut();
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut RW {
        self.inner.as_mut().unwrap()
    }

    /// Returns a reference to the internally buffered writer data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nameless::BufReaderWriter;
    /// use std::net::TcpStream;
    ///
    /// let buf_writer = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // See how many bytes are currently buffered
    /// let bytes_buffered = buf_writer.writer_buffer().len();
    /// ```
    #[inline]
    pub fn writer_buffer(&self) -> &[u8] {
        &self.writer_buf
    }

    /// Returns a reference to the internally buffered reader data.
    ///
    /// Unlike [`fill_buf`], this will not attempt to fill the buffer if it is empty.
    ///
    /// [`fill_buf`]: BufRead::fill_buf
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io::BufRead;
    /// use std::fs::File;
    /// use nameless::BufReaderWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let f = File::open("log.txt")?;
    ///     let mut reader = BufReaderWriter::new(f);
    ///     assert!(reader.reader_buffer().is_empty());
    ///
    ///     if reader.fill_buf()?.len() > 0 {
    ///         assert!(!reader.reader_buffer().is_empty());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn reader_buffer(&self) -> &[u8] {
        &self.reader_buf[self.pos..self.cap]
    }

    /// Returns the number of bytes the internal writer buffer can hold without flushing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::TcpStream;
    /// use nameless::BufReaderWriter;
    ///
    /// let buf_reader_writer = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // Check the capacity of the inner buffer
    /// let capacity = buf_reader_writer.writer_capacity();
    /// // Calculate how many bytes can be written without flushing
    /// let without_flush = capacity - buf_reader_writer.writer_buffer().len();
    /// ```
    #[inline]
    pub fn writer_capacity(&self) -> usize {
        self.writer_buf.capacity()
    }

    /// Returns the number of bytes the internal reader buffer can hold at once.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io::BufRead;
    /// use std::fs::File;
    /// use nameless::BufReaderWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let f = File::open("log.txt")?;
    ///     let mut reader = BufReaderWriter::new(f);
    ///
    ///     let capacity = reader.reader_capacity();
    ///     let buffer = reader.fill_buf()?;
    ///     assert!(buffer.len() <= capacity);
    ///     Ok(())
    /// }
    /// ```
    pub fn reader_capacity(&self) -> usize {
        self.reader_buf.len()
    }

    /// Unwraps this `BufReaderWriter<RW>`, returning the underlying reader/writer.
    ///
    /// The buffer is written out before returning the reader/writer.
    ///
    /// # Errors
    ///
    /// An [`Err`] will be returned if an error occurs while flushing the buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::TcpStream;
    /// use nameless::BufReaderWriter;
    ///
    /// let mut buffer = BufReaderWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // unwrap the TcpStream and flush the buffer
    /// let stream = buffer.into_inner().unwrap();
    /// ```
    pub fn into_inner(mut self) -> Result<RW, IntoInnerError<Self>> {
        match self.flush_buf() {
            Err(e) => Err(IntoInnerError::new(self, e)),
            Ok(()) => Ok(self.inner.take().unwrap()),
        }
    }

    /// Invalidates all data in the internal buffer.
    #[inline]
    fn discard_reader_buffer(&mut self) {
        self.pos = 0;
        self.cap = 0;
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
            bufs.iter().for_each(|b| self.writer_buf.extend_from_slice(b));
            Ok(total_len)
        }
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.get_ref().is_write_vectored()
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf().and_then(|()| self.get_mut().flush())
    }
}

impl<RW: ReadWrite> Read for BufReaderWriter<RW> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.pos == self.cap && buf.len() >= self.reader_buf.len() {
            self.discard_reader_buffer();
            return self.inner.as_mut().unwrap().read(buf);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let total_len = bufs.iter().map(|b| b.len()).sum::<usize>();
        if self.pos == self.cap && total_len >= self.reader_buf.len() {
            self.discard_reader_buffer();
            return self.inner.as_mut().unwrap().read_vectored(bufs);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read_vectored(bufs)?
        };
        self.consume(nread);
        Ok(nread)
    }

    #[cfg(feature = "nightly")]
    fn is_read_vectored(&self) -> bool {
        self.inner.as_ref().unwrap().is_read_vectored()
    }

    // we can't skip unconditionally because of the large buffer case in read.
    #[cfg(feature = "nightly")]
    unsafe fn initializer(&self) -> Initializer {
        self.inner.as_ref().unwrap().initializer()
    }
}

impl<RW: ReadWrite> BufRead for BufReaderWriter<RW> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // If we've reached the end of our internal buffer then we need to fetch
        // some more data from the underlying reader.
        // Branch using `>=` instead of the more correct `==`
        // to tell the compiler that the pos..cap slice is always valid.
        if self.pos >= self.cap {
            debug_assert!(self.pos == self.cap);
            self.cap = self.inner.as_mut().unwrap().read(&mut self.reader_buf)?;
            self.pos = 0;
        }
        Ok(&self.reader_buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.cap);
    }
}

impl<RW: ReadWrite> fmt::Debug for BufReaderWriter<RW>
where
    RW: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BufReaderWriter")
            .field("inner", &self.inner.as_ref().unwrap())
            .field("reader_buffer", &format_args!("{}/{}", self.cap - self.pos, self.reader_buf.len()))
            .field("writer_buffer", &format_args!("{}/{}", self.writer_buf.len(), self.writer_buf.capacity()))
            .finish()
    }
}

impl<RW: ReadWrite> Drop for BufReaderWriter<RW> {
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            // dtors should not panic, so we ignore a failed flush
            let _r = self.flush_buf();
        }
    }
}
