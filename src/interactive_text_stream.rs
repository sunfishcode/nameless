use crate::{
    open_interactive::{open_interactive, Interactive},
    Pseudonym,
};
use basic_text::TextDuplexer;
use clap::TryFromOsArg;
use duplex::Duplex;
use io_streams::StreamDuplexer;
use layered_io::{Bufferable, LayeredDuplexer, ReadLayered, Status, WriteLayered};
use std::{
    ffi::OsStr,
    fmt::{self, Arguments, Debug, Formatter},
    io::{self, IoSlice, IoSliceMut, Read, Write},
};
use terminal_io::{
    DuplexTerminal, ReadTerminal, Terminal, TerminalColorSupport, TerminalDuplexer, WriteTerminal,
};
use utf8_io::{ReadStr, ReadStrLayered, Utf8Duplexer, WriteStr};

/// An `InteractiveTextStream` implements `Read` and `Write` as is meant
/// to be used with interactive streams.
///
/// The primary way to construct an `InteractiveTextStream` is to use it as
/// a type in a `kommand` argument or `clap_derive` struct. Command-line
/// arguments will then be automatically converted into input streams.
/// Currently supported syntaxes include:
///  - Names starting with `connect:` or `accept:`, which are
///    interpreted as socket addresses to connect to or accept from.
///    Socket addresses may contain host:port pairs or, on platforms which
///    support it, filesystem paths to Unix-domain sockets.
///  - "-" is interpreted as the pair (stdin, stdout).
///  - "(...)" runs a command with pipes to and from the child process'
///    (stdin, stdout), on platforms whch support it.
pub struct InteractiveTextStream {
    name: String,
    duplexer: TextDuplexer<Utf8Duplexer<LayeredDuplexer<TerminalDuplexer<StreamDuplexer>>>>,
}

impl InteractiveTextStream {
    /// Write the given `Pseudonym` to the output stream.
    #[inline]
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        Write::write_all(self, pseudonym.name.as_bytes())
    }

    /// Write the name of the given output stream to the output stream. This is
    /// needed because the name of an `InteractiveTextStream` is not available
    /// in the public API.
    #[inline]
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    fn from_interactive(interactive: Interactive) -> Self {
        let duplexer = TerminalDuplexer::with_handle(interactive.duplexer);
        let duplexer = LayeredDuplexer::new(duplexer);
        let duplexer = Utf8Duplexer::new(duplexer);
        let duplexer = TextDuplexer::new(duplexer);
        Self {
            name: interactive.name,
            duplexer,
        }
    }
}

/// Implement `FromStr` so that `clap_derive` can parse `InteractiveTextStream`
/// arguments automatically.
///
/// This is hidden from the documentation as it opens resources from
/// strings using ambient authorities.
#[doc(hidden)]
impl TryFromOsArg for InteractiveTextStream {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_os_str_arg(os: &OsStr) -> anyhow::Result<Self> {
        open_interactive(os).map(Self::from_interactive)
    }
}

impl ReadLayered for InteractiveTextStream {
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

impl Read for InteractiveTextStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.duplexer.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.duplexer.read_vectored(bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.duplexer.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.duplexer.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.duplexer.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.duplexer.read_exact(buf)
    }
}

impl WriteLayered for InteractiveTextStream {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.duplexer.close()
    }
}

impl WriteStr for InteractiveTextStream {
    #[inline]
    fn write_str(&mut self, buf: &str) -> io::Result<()> {
        self.duplexer.write_str(buf)
    }
}

impl Write for InteractiveTextStream {
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

impl Terminal for InteractiveTextStream {}

impl ReadTerminal for InteractiveTextStream {
    #[inline]
    fn is_line_by_line(&self) -> bool {
        self.duplexer.is_line_by_line()
    }

    #[inline]
    fn is_input_terminal(&self) -> bool {
        self.duplexer.is_input_terminal()
    }
}

impl WriteTerminal for InteractiveTextStream {
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

impl DuplexTerminal for InteractiveTextStream {}

impl Duplex for InteractiveTextStream {}

impl Bufferable for InteractiveTextStream {
    #[inline]
    fn abandon(&mut self) {
        self.duplexer.abandon()
    }
}

impl ReadStr for InteractiveTextStream {
    #[inline]
    fn read_str(&mut self, buf: &mut str) -> io::Result<usize> {
        self.duplexer.read_str(buf)
    }

    #[inline]
    fn read_exact_str(&mut self, buf: &mut str) -> io::Result<()> {
        self.duplexer.read_exact_str(buf)
    }
}

impl ReadStrLayered for InteractiveTextStream {
    #[inline]
    fn read_str_with_status(&mut self, buf: &mut str) -> io::Result<(usize, Status)> {
        self.duplexer.read_str_with_status(buf)
    }

    #[inline]
    fn read_exact_str_using_status(&mut self, buf: &mut str) -> io::Result<Status> {
        self.duplexer.read_exact_str_using_status(buf)
    }
}

impl Debug for InteractiveTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InteractiveTextStream");
        b.field("duplexer", &self.duplexer);
        b.finish()
    }
}
