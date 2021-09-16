use crate::open_interactive::{open_interactive, Interactive};
use crate::Pseudonym;
use clap::TryFromOsArg;
use duplex::Duplex;
use io_streams::StreamDuplexer;
use layered_io::{
    default_read, default_read_to_end, default_read_to_string, default_read_vectored, Bufferable,
    LayeredDuplexer, ReadLayered, Status, WriteLayered,
};
use std::ffi::OsStr;
use std::fmt::{self, Arguments, Debug, Formatter};
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use terminal_io::{
    DuplexTerminal, NeverTerminalDuplexer, ReadTerminal, Terminal, TerminalColorSupport,
    WriteTerminal,
};

/// An `InteractiveByteStream` implements `Read` and `Write` as is meant
/// to be used with interactive streams.
///
/// The primary way to construct an `InteractiveByteStream` is to use it as
/// a type in a `kommand` argument or `clap_derive` struct. Command-line
/// arguments will then be automatically converted into input streams.
/// Currently supported syntaxes include:
///  - Names starting with `connect:` or `accept:`, which are interpreted as
///    socket addresses to connect to or accept from. Socket addresses may
///    contain host:port pairs or, on platforms which support it, filesystem
///    paths to Unix-domain sockets.
///  - "-" is interpreted as the pair (stdin, stdout).
///  - "(...)" runs a command with pipes to and from the child process' (stdin,
///    stdout), on platforms whch support it.
pub struct InteractiveByteStream {
    name: String,
    duplexer: LayeredDuplexer<NeverTerminalDuplexer<StreamDuplexer>>,
}

impl InteractiveByteStream {
    /// Return a `Pseudonym` which encapsulates this stream's name (typically
    /// its filesystem path or its URL). This allows it to be written to an
    /// `InteractiveByteStream` while otherwise remaining entirely opaque.
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    fn from_interactive(interactive: Interactive) -> Self {
        let duplexer = NeverTerminalDuplexer::new(interactive.duplexer);
        let duplexer = LayeredDuplexer::new(duplexer);
        Self {
            name: interactive.name,
            duplexer,
        }
    }
}

/// Implement `TryFromOsArg` so that `clap_derive` can parse
/// `InteractiveByteStream` arguments automatically.
///
/// This is hidden from the documentation as it opens resources from
/// strings using ambient authorities.
#[doc(hidden)]
impl TryFromOsArg for InteractiveByteStream {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_os_str_arg(os: &OsStr) -> anyhow::Result<Self> {
        open_interactive(os).map(Self::from_interactive)
    }
}

impl ReadLayered for InteractiveByteStream {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        self.duplexer.read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        self.duplexer.read_vectored_with_status(bufs)
    }
}

impl Read for InteractiveByteStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        default_read(self, buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        default_read_vectored(self, bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.duplexer.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        default_read_to_end(self, buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        default_read_to_string(self, buf)
    }
}

impl WriteLayered for InteractiveByteStream {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.duplexer.close()
    }
}

impl Write for InteractiveByteStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.duplexer.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.duplexer.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.duplexer.write_vectored(bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.duplexer.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.duplexer.write_all(buf)
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.duplexer.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.duplexer.write_fmt(fmt)
    }
}

impl Bufferable for InteractiveByteStream {
    #[inline]
    fn abandon(&mut self) {
        self.duplexer.abandon()
    }
}

impl Terminal for InteractiveByteStream {}

impl ReadTerminal for InteractiveByteStream {
    #[inline]
    fn is_line_by_line(&self) -> bool {
        self.duplexer.is_line_by_line()
    }

    #[inline]
    fn is_input_terminal(&self) -> bool {
        self.duplexer.is_input_terminal()
    }
}

impl WriteTerminal for InteractiveByteStream {
    #[inline]
    fn color_support(&self) -> TerminalColorSupport {
        self.duplexer.color_support()
    }

    #[inline]
    fn color_preference(&self) -> bool {
        self.duplexer.color_preference()
    }

    #[inline]
    fn is_output_terminal(&self) -> bool {
        self.duplexer.is_output_terminal()
    }
}

impl DuplexTerminal for InteractiveByteStream {}

impl Duplex for InteractiveByteStream {}

impl Debug for InteractiveByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InteractiveByteStream");
        b.finish()
    }
}
