use crate::{
    clap::TryFromOsArg,
    lazy_output::FromLazyOutput,
    open_output::{open_output, Output},
    Pseudonym, Type,
};
use anyhow::anyhow;
use io_streams::StreamWriter;
use layered_io::{Bufferable, LayeredWriter, WriteLayered};
use std::{
    ffi::{OsStr, OsString},
    fmt::{self, Arguments, Debug, Formatter},
    io::{self, IoSlice, Write},
};
use terminal_io::{NeverTerminalWriter, TerminalWriter, WriteTerminal};

/// An output stream for binary output.
///
/// An `OutputByteStream` implements `Write` so it supports `write`,
/// `write_all`, etc. and can be used anywhere a `Write`-implementing
/// object is needed.
///
/// `OutputByteStream` is unbuffered (even when it is stdout), so wrapping
/// it in a [`std::io::BufWriter`] or [`std::io::LineWriter`] is
/// recommended for performance.
///
/// The primary way to construct an `OutputByteStream` is to use it as
/// a type in a `StructOpt` struct. Command-line arguments will then
/// be automatically converted into output streams. Currently supported
/// syntaxes include:
///  - Names starting with `file:` are interpreted as local filesystem
///    URLs providing paths to files to open.
///  - "-" is interpreted as standard output.
///  - "(...)" runs a command with a pipe to the child process' stdin,
///    on platforms whch support it.
///  - Names which don't parse as URLs are interpreted as plain local
///    filesystem paths. To force a string to be interpreted as a plain
///    local path, arrange for it to begin with `./` or `/`.
///
/// Programs using `OutputByteStream` as an argument should avoid using
/// `std::io::stdout`, `std::println`, or anything else which uses standard
/// output implicitly.
pub struct OutputByteStream {
    name: String,
    writer: LayeredWriter<NeverTerminalWriter<StreamWriter>>,
    type_: Type,
}

impl OutputByteStream {
    /// Write the given `Pseudonym` to the output stream.
    #[inline]
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        Write::write_all(self, pseudonym.name.as_bytes())
    }

    /// Write the name of the given output stream to the output stream. This is
    /// needed because the name of an `OutputByteStream` is not available in
    /// the public API.
    #[inline]
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    /// If the output stream metadata implies a particular media type, also
    /// known as MIME type, return it. Some output streams know their type,
    /// though many do not.
    #[inline]
    pub fn type_(&self) -> &Type {
        &self.type_
    }

    fn from_output(output: Output) -> anyhow::Result<Self> {
        let writer = NeverTerminalWriter::new(output.writer);

        let writer = TerminalWriter::with_handle(writer);
        if writer.is_output_terminal() {
            return Err(anyhow!("attempted to write binary output to a terminal"));
        }

        let writer = LayeredWriter::new(writer.into_inner());

        Ok(Self {
            name: output.name,
            writer,
            type_: output.type_,
        })
    }
}

/// Implement `From<&OsStr>` so that `structopt` can parse `OutputByteStream`
/// objects automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl TryFromOsArg for OutputByteStream {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_os_str_arg(os: &OsStr) -> anyhow::Result<Self> {
        open_output(os, Type::unknown()).and_then(Self::from_output)
    }
}

impl WriteLayered for OutputByteStream {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.writer.close()
    }
}

impl Write for OutputByteStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.writer.write_vectored(bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.writer.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.writer.write_all(buf)
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.writer.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.writer.write_fmt(fmt)
    }
}

impl Bufferable for OutputByteStream {
    #[inline]
    fn abandon(&mut self) {
        self.writer.abandon()
    }
}

impl FromLazyOutput for OutputByteStream {
    type Err = anyhow::Error;

    fn from_lazy_output(name: OsString, type_: Type) -> Result<Self, anyhow::Error> {
        open_output(&name, type_).and_then(Self::from_output)
    }
}

impl Debug for OutputByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("OutputByteStream");
        b.field("type_", &self.type_);
        b.finish()
    }
}
