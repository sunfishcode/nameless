use crate::open_input::{open_input, Input};
use crate::{MediaType, Pseudonym};
use basic_text::{ReadText, ReadTextLayered, TextReader, TextSubstr};
use clap::{AmbientAuthority, TryFromOsArg};
use io_streams::StreamReader;
use layered_io::{Bufferable, LayeredReader, ReadLayered, Status};
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, IoSliceMut, Read};
use terminal_io::TerminalReader;
use utf8_io::{ReadStr, ReadStrLayered, Utf8Reader};

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
/// a type in a `kommand` argument or `clap_derive` struct. Command-line
/// arguments will then be automatically converted into input streams.
/// Currently supported syntaxes include:
///  - Names starting with `https:` or `http:`, which are interpreted as URLs
///    to open.
///  - Names starting with `data:` are interpreted as data URLs proving the
///    data in their payload.
///  - Names starting with `file:` are interpreted as local filesystem URLs
///    providing paths to files to open.
///  - "-" is interpreted as standard input.
///  - "(...)" runs a command with a pipe from the child process' stdout, on
///    platforms whch support it.
///  - Names which don't parse as URLs are interpreted as plain local
///    filesystem paths. To force a string to be interpreted as a plain local
///    path, arrange for it to begin with `./` or `/`.
pub struct InputTextStream {
    name: String,
    reader: TextReader<Utf8Reader<LayeredReader<TerminalReader<StreamReader>>>>,
    media_type: MediaType,
    initial_size: Option<u64>,
}

impl InputTextStream {
    /// If the input stream metadata implies a particular media type, also
    /// known as MIME type, return it. Many input streams know their type,
    /// though some do not. This is strictly based on available metadata, and
    /// not on examining any of the contents of the stream, and there's no
    /// guarantee the contents are valid.
    pub fn media_type(&self) -> &MediaType {
        &self.media_type
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

    fn from_input(input: Input) -> Self {
        let reader = TerminalReader::with_handle(input.reader);
        let reader = TextReader::new(reader);
        let media_type = input.media_type.union(MediaType::text());
        Self {
            name: input.name,
            reader,
            media_type,
            initial_size: input.initial_size,
        }
    }
}

/// Implement `TryFromOsArg` so that `clap_derive` can parse `InputTextStream`
/// arguments automatically.
///
/// This is hidden from the documentation as it opens resources from
/// strings using ambient authorities.
#[doc(hidden)]
impl TryFromOsArg for InputTextStream {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_os_str_arg(
        os: &OsStr,
        ambient_authority: AmbientAuthority,
    ) -> anyhow::Result<Self> {
        open_input(os, ambient_authority).map(Self::from_input)
    }
}

impl ReadLayered for InputTextStream {
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
    fn read_str(&mut self, buf: &mut str) -> io::Result<usize> {
        self.reader.read_str(buf)
    }
}

impl ReadStrLayered for InputTextStream {
    #[inline]
    fn read_str_with_status(&mut self, buf: &mut str) -> io::Result<(usize, Status)> {
        self.reader.read_str_with_status(buf)
    }
}

impl ReadText for InputTextStream {
    #[inline]
    fn read_text_substr(&mut self, buf: &mut TextSubstr) -> io::Result<usize> {
        self.reader.read_text_substr(buf)
    }

    #[inline]
    fn read_exact_text_substr(&mut self, buf: &mut TextSubstr) -> io::Result<()> {
        self.reader.read_exact_text_substr(buf)
    }
}

impl ReadTextLayered for InputTextStream {
    #[inline]
    fn read_text_substr_with_status(
        &mut self,
        buf: &mut TextSubstr,
    ) -> io::Result<(usize, Status)> {
        self.reader.read_text_substr_with_status(buf)
    }

    #[inline]
    fn read_exact_text_substr_using_status(&mut self, buf: &mut TextSubstr) -> io::Result<Status> {
        self.reader.read_exact_text_substr_using_status(buf)
    }
}

impl Debug for InputTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InputTextStream");
        b.field("media_type", &self.media_type);
        b.field("initial_size", &self.initial_size);
        b.finish()
    }
}

#[test]
fn data_url_plain() {
    let mut s = String::new();
    InputTextStream::try_from_os_str_arg(
        "data:,Hello%2C%20World!".as_ref(),
        clap::ambient_authority(),
    )
    .unwrap()
    .read_to_string(&mut s)
    .unwrap();
    assert_eq!(s, "Hello, World!\n");
}

#[test]
fn data_url_base64() {
    let mut s = String::new();
    InputTextStream::try_from_os_str_arg(
        "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==".as_ref(),
        clap::ambient_authority(),
    )
    .unwrap()
    .read_to_string(&mut s)
    .unwrap();
    assert_eq!(s, "Hello, World!\n");
}
