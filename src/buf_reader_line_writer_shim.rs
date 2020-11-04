//! This file is derived from Rust's library/std/src/io/buffered at revision
//! a78a62fc996ba16f7a111c99520b23f77029f4eb.

use crate::{BufReaderWriter, ReadWrite};
use std::io::{self, IoSlice, Write};

#[derive(Debug)]
pub(crate) struct BufReaderLineWriterShim<'a, RW: ReadWrite> {
    buffer: &'a mut BufReaderWriter<RW>,
}

impl<'a, RW: ReadWrite> BufReaderLineWriterShim<'a, RW> {
    pub fn new(buffer: &'a mut BufReaderWriter<RW>) -> Self {
        Self { buffer }
    }

    fn inner_mut(&mut self) -> &mut RW {
        self.buffer.get_mut()
    }

    fn writer_buffered(&self) -> &[u8] {
        self.buffer.writer_buffer()
    }

    fn flush_if_completed_line(&mut self) -> io::Result<()> {
        match self.writer_buffered().last().copied() {
            Some(b'\n') => self.buffer.flush_buf(),
            _ => Ok(()),
        }
    }
}

impl<'a, RW: ReadWrite> Write for BufReaderLineWriterShim<'a, RW> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let newline_idx = match memchr::memrchr(b'\n', buf) {
            // If there are no new newlines (that is, if this write is less than
            // one line), just do a regular buffered write (which may flush if
            // we exceed the inner buffer's size)
            None => {
                self.flush_if_completed_line()?;
                return self.buffer.write(buf);
            }
            // Otherwise, arrange for the lines to be written directly to the
            // inner writer.
            Some(newline_idx) => newline_idx + 1,
        };

        // Flush existing content to prepare for our write. We have to do this
        // before attempting to write `buf` in order to maintain consistency;
        // if we add `buf` to the buffer then try to flush it all at once,
        // we're obligated to return Ok(), which would mean suppressing any
        // errors that occur during flush.
        self.buffer.flush_buf()?;

        // This is what we're going to try to write directly to the inner
        // writer. The rest will be buffered, if nothing goes wrong.
        let lines = &buf[..newline_idx];

        // Write `lines` directly to the inner writer. In keeping with the
        // `write` convention, make at most one attempt to add new (unbuffered)
        // data. Because this write doesn't touch the `BufReaderWriter` state directly,
        // and the buffer is known to be empty, we don't need to worry about
        // self.buffer.panicked here.
        let flushed = self.inner_mut().write(lines)?;

        // If buffer returns Ok(0), propagate that to the caller without
        // doing additional buffering; otherwise we're just guaranteeing
        // an "ErrorKind::WriteZero" later.
        if flushed == 0 {
            return Ok(0);
        }

        // Now that the write has succeeded, buffer the rest (or as much of
        // the rest as possible). If there were any unwritten newlines, we
        // only buffer out to the last unwritten newline that fits in the
        // buffer; this helps prevent flushing partial lines on subsequent
        // calls to BufReaderLineWriterShim::write.

        // Handle the cases in order of most-common to least-common, under
        // the presumption that most writes succeed in totality, and that most
        // writes are smaller than the buffer.
        // - Is this a partial line (ie, no newlines left in the unwritten tail)
        // - If not, does the data out to the last unwritten newline fit in
        //   the buffer?
        // - If not, scan for the last newline that *does* fit in the buffer
        let tail = if flushed >= newline_idx {
            &buf[flushed..]
        } else if newline_idx - flushed <= self.buffer.writer_capacity() {
            &buf[flushed..newline_idx]
        } else {
            let scan_area = &buf[flushed..];
            let scan_area = &scan_area[..self.buffer.writer_capacity()];
            match memchr::memrchr(b'\n', scan_area) {
                Some(newline_idx) => &scan_area[..newline_idx + 1],
                None => scan_area,
            }
        };

        let buffered = self.buffer.write_to_buf(tail);
        Ok(flushed + buffered)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        // If there's no specialized behavior for write_vectored, just use
        // write. This has the benefit of more granular partial-line handling.
        #[cfg(feature = "nightly")]
        if !self.is_write_vectored() {
            return match bufs.iter().find(|buf| !buf.is_empty()) {
                Some(buf) => self.write(buf),
                None => Ok(0),
            };
        }

        // Find the buffer containing the last newline
        let last_newline_buf_idx = bufs
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, buf)| memchr::memchr(b'\n', buf).map(|_| i));

        // If there are no new newlines (that is, if this write is less than
        // one line), just do a regular buffered write
        let last_newline_buf_idx = match last_newline_buf_idx {
            // No newlines; just do a normal buffered write
            None => {
                self.flush_if_completed_line()?;
                return self.buffer.write_vectored(bufs);
            }
            Some(i) => i,
        };

        // Flush existing content to prepare for our write
        self.buffer.flush_buf()?;

        // This is what we're going to try to write directly to the inner
        // writer. The rest will be buffered, if nothing goes wrong.
        let (lines, tail) = bufs.split_at(last_newline_buf_idx + 1);

        // Write `lines` directly to the inner writer. In keeping with the
        // `write` convention, make at most one attempt to add new (unbuffered)
        // data. Because this write doesn't touch the BufReaderWriter state directly,
        // and the buffer is known to be empty, we don't need to worry about
        // self.panicked here.
        let flushed = self.inner_mut().write_vectored(lines)?;

        // If inner returns Ok(0), propagate that to the caller without
        // doing additional buffering; otherwise we're just guaranteeing
        // an "ErrorKind::WriteZero" later.
        if flushed == 0 {
            return Ok(0);
        }

        // Don't try to reconstruct the exact amount written; just bail
        // in the event of a partial write
        let lines_len = lines.iter().map(|buf| buf.len()).sum();
        if flushed < lines_len {
            return Ok(flushed);
        }

        // Now that the write has succeeded, buffer the rest (or as much of the
        // rest as possible)
        let buffered: usize = tail
            .iter()
            .filter(|buf| !buf.is_empty())
            .map(|buf| self.buffer.write_to_buf(buf))
            .take_while(|&n| n > 0)
            .sum();

        Ok(flushed + buffered)
    }

    #[cfg(feature = "nightly")]
    fn is_write_vectored(&self) -> bool {
        self.buffer.is_write_vectored()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match memchr::memrchr(b'\n', buf) {
            // If there are no new newlines (that is, if this write is less than
            // one line), just do a regular buffered write (which may flush if
            // we exceed the inner buffer's size)
            None => {
                self.flush_if_completed_line()?;
                self.buffer.write_all(buf)
            }
            Some(newline_idx) => {
                let (lines, tail) = buf.split_at(newline_idx + 1);

                if self.writer_buffered().is_empty() {
                    self.inner_mut().write_all(lines)?;
                } else {
                    // If there is any buffered data, we add the incoming lines
                    // to that buffer before flushing, which saves us at least
                    // one write call. We can't really do this with `write`,
                    // since we can't do this *and* not suppress errors *and*
                    // report a consistent state to the caller in a return
                    // value, but here in write_all it's fine.
                    self.buffer.write_all(lines)?;
                    self.buffer.flush_buf()?;
                }

                self.buffer.write_all(tail)
            }
        }
    }
}
