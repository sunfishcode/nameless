//! This file is derived from Rust's library/std/src/io/buffered at revision
//! f7801d6c7cc19ab22bdebcc8efa894a564c53469.

use std::fmt;
use std::io::{self, IoSlice, Write};
use super::{BufWriter, IntoInnerError, LineWriterShim};
use crate::ReadWrite;

/// Wraps a writer and buffers output to it, flushing whenever a newline
/// (`0x0a`, `'\n'`) is detected.
///
/// The [`BufWriter`] struct wraps a writer and buffers its output.
/// But it only does this batched write when it goes out of scope, or when the
/// internal buffer is full. Sometimes, you'd prefer to write each line as it's
/// completed, rather than the entire buffer at once. Enter `LineWriter`. It
/// does exactly that.
///
/// Like [`BufWriter`], a `LineWriter`’s buffer will also be flushed when the
/// `LineWriter` goes out of scope or when its internal buffer is full.
///
/// If there's still a partial line in the buffer when the `LineWriter` is
/// dropped, it will flush those contents.
///
/// # Examples
///
/// We can use `LineWriter` to write one line at a time, significantly
/// reducing the number of actual writes to the file.
///
/// ```no_run
/// use std::fs::{self, File};
/// use std::io::prelude::*;
/// use std::io::LineWriter;
///
/// fn main() -> std::io::Result<()> {
///     let road_not_taken = b"I shall be telling this with a sigh
/// Somewhere ages and ages hence:
/// Two roads diverged in a wood, and I -
/// I took the one less traveled by,
/// And that has made all the difference.";
///
///     let file = File::create("poem.txt")?;
///     let mut file = LineWriter::new(file);
///
///     file.write_all(b"I shall be telling this with a sigh")?;
///
///     // No bytes are written until a newline is encountered (or
///     // the internal buffer is filled).
///     assert_eq!(fs::read_to_string("poem.txt")?, "");
///     file.write_all(b"\n")?;
///     assert_eq!(
///         fs::read_to_string("poem.txt")?,
///         "I shall be telling this with a sigh\n",
///     );
///
///     // Write the rest of the poem.
///     file.write_all(b"Somewhere ages and ages hence:
/// Two roads diverged in a wood, and I -
/// I took the one less traveled by,
/// And that has made all the difference.")?;
///
///     // The last line of the poem doesn't end in a newline, so
///     // we have to flush or drop the `LineWriter` to finish
///     // writing.
///     file.flush()?;
///
///     // Confirm the whole poem was written.
///     assert_eq!(fs::read("poem.txt")?, &road_not_taken[..]);
///     Ok(())
/// }
/// ```
pub struct LineWriter<RW: ReadWrite> {
    inner: BufWriter<RW>,
}

impl<RW: ReadWrite> LineWriter<RW> {
    /// Creates a new `LineWriter`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let file = LineWriter::new(file);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(inner: RW) -> Self {
        // Lines typically aren't that long, don't use a giant buffer
        Self::with_capacity(1024, inner)
    }

    /// Creates a new `LineWriter` with a specified capacity for the internal
    /// buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let file = LineWriter::with_capacity(100, file);
    ///     Ok(())
    /// }
    /// ```
    pub fn with_capacity(capacity: usize, inner: RW) -> Self {
        Self { inner: BufWriter::with_capacity(capacity, inner) }
    }

    /// Gets a reference to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let file = LineWriter::new(file);
    ///
    ///     let reference = file.get_ref();
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn get_ref(&self) -> &RW {
        self.inner.get_ref()
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// Caution must be taken when calling methods on the mutable reference
    /// returned as extra writes could corrupt the output stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let mut file = LineWriter::new(file);
    ///
    ///     // we can use reference just like file
    ///     let reference = file.get_mut();
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut RW {
        self.inner.get_mut()
    }

    /// Unwraps this `LineWriter`, returning the underlying writer.
    ///
    /// The internal buffer is written out before returning the writer.
    ///
    /// # Errors
    ///
    /// An [`Err`] will be returned if an error occurs while flushing the buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///
    ///     let writer: LineWriter<File> = LineWriter::new(file);
    ///
    ///     let file: File = writer.into_inner()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn into_inner(self) -> Result<RW, IntoInnerError<Self>> {
        self.inner.into_inner().map_err(|err| err.new_wrapped(|inner| Self { inner }))
    }
}

impl<RW: ReadWrite> Write for LineWriter<RW> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        LineWriterShim::new(&mut self.inner).write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        LineWriterShim::new(&mut self.inner).write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        LineWriterShim::new(&mut self.inner).write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        LineWriterShim::new(&mut self.inner).write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        LineWriterShim::new(&mut self.inner).write_fmt(fmt)
    }
}

impl<RW: ReadWrite> fmt::Debug for LineWriter<RW>
where
    RW: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("LineWriter")
            .field("inner", &self.get_ref())
            .field(
                "writer_buffer",
                &format_args!("{}/{}", self.inner.buffer().len(), self.inner.capacity()),
            )
            .finish()
    }
}
