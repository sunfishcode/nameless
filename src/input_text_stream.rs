use crate::{InputByteStream, Pseudonym, Type};
use io_ext::{Bufferable, ReadExt, Status};
use io_ext_adapters::ExtReader;
use io_handles::ReadHandle;
use std::{
    fmt::{self, Debug, Formatter},
    io::{self, IoSliceMut, Read},
    str::FromStr,
};
use terminal_support::TerminalReader;
use text_streams::{TextReader, ReadStr};

/// In input stream for plain text input.
///
/// An `InputTextStream` implements `Read` so it supports `read`,
/// `read_to_end`, `read_to_str`, etc. and can be used anywhere a
/// `Read`-implementing object is needed.
///
/// `InputTextStream` is unbuffered (even when it is stdin), so wrapping
/// it in a [`std::io::BufReader`] is recommended for performance and
/// ease of use.
///
/// The primary way to construct an `InputTextStream` is to use it as
/// a type in a `StructOpt` struct. Command-line arguments will then
/// be automatically converted into input streams. Currently supported
/// syntaxes include:
///  - Names starting with `https:` or `http:`, which are interpreted
///    as URLs to open.
///  - Names starting with `data:` are interpreted as data URLs proving
///    the data in their payload.
///  - Names starting with `file:` are interpreted as local filesystem
///    URLs providing paths to files to open.
///  - "-" is interpreted as standard input.
///  - "(...)" runs a command with a pipe from the child process' stdout,
///    on platforms whch support it.
///  - Names which don't parse as URLs are interpreted as plain local
///    filesystem paths. To force a string to be interpreted as a plain
///    local path, arrange for it to begin with `./` or `/`.
pub struct InputTextStream {
    name: String,
    reader: TextReader<ExtReader<TerminalReader<ReadHandle>>>,
    type_: Type,
    initial_size: Option<u64>,
}

impl InputTextStream {
    /// If the input stream metadata implies a particular media type, also
    /// known as MIME type, return it. Many input streams know their type,
    /// though some do not. This is strictly based on available metadata, and
    /// not on examining any of the contents of the stream, and there's no
    /// guarantee the contents are valid.
    pub fn type_(&self) -> &Type {
        &self.type_
    }

    /// Return the initial size of the stream, in bytes. This is strictly based
    /// on available metadata, and not on examining any of the contents of the
    /// stream, and the stream could end up being shorter or longer if the
    /// source is concurrently modified or it produces content which must be
    /// adapted to meet the "plain text" requirements.
    pub fn initial_size(&self) -> Option<u64> {
        self.initial_size
    }

    /// Return a `Pseudonym` which encapsulates this stream's name (typically
    /// its filesystem path or its URL). This allows it to be written to an
    /// `OutputByteStream` while otherwise remaining entirely opaque.
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    /// Return an input byte stream representing standard input.
    pub fn stdin() -> anyhow::Result<Self> {
        let stdin = InputByteStream::stdin()?;
        let (name, reader, type_, initial_size) = stdin.into_parts();
        let reader = reader.abandon_into_inner().unwrap().into_inner();
        let reader = TerminalReader::with_handle(reader);
        let reader = ExtReader::new(reader);
        let reader = TextReader::new(reader);
        Ok(Self {
            name,
            reader,
            type_,
            initial_size,
        })
    }

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let reader = InputByteStream::from_str(s)?;
        let (name, reader, type_, initial_size) = reader.into_parts();
        let reader = reader.abandon_into_inner().unwrap().into_inner();
        let reader = TerminalReader::with_handle(reader);
        let reader = ExtReader::new(reader);
        let reader = TextReader::new(reader);
        Ok(Self {
            name,
            reader,
            type_,
            initial_size,
        })
    }
}

/// Implement `FromStr` so that `structopt` can parse `InputTextStream`
/// arguments automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl FromStr for InputTextStream {
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

impl ReadExt for InputTextStream {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        self.reader.read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        self.reader.read_vectored_with_status(bufs)
    }
}

impl Read for InputTextStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.reader.read_vectored(bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.reader.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.reader.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.reader.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reader.read_exact(buf)
    }
}

impl Bufferable for InputTextStream {
    #[inline]
    fn abandon(&mut self) {
        self.reader.abandon()
    }
}

impl ReadStr for InputTextStream {
    #[inline]
    fn read_str(&mut self, buf: &mut str) -> io::Result<(usize, Status)> {
        self.reader.read_str(buf)
    }
}

impl Debug for InputTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InputTextStream");
        b.field("type_", &self.type_);
        b.field("initial_size", &self.initial_size);
        b.finish()
    }
}

#[test]
fn data_url_plain() {
    let mut s = String::new();
    InputTextStream::from_str("data:,Hello%2C%20World!")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "Hello, World!\n");
}

#[test]
fn data_url_base64() {
    let mut s = String::new();
    InputTextStream::from_str("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "Hello, World!\n");
}
