#[cfg(not(windows))]
use crate::path_to_name::path_to_name;
use crate::{InteractiveByteStream, Pseudonym};
use anyhow::anyhow;
use basic_text::TextDuplexer;
use char_device::CharDevice;
use duplex::Duplex;
use io_streams::StreamDuplexer;
use layered_io::{Bufferable, LayeredDuplexer, ReadLayered, Status, WriteLayered};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(windows)]
use std::os::windows::io::FromRawHandle;
use std::{
    fmt::{self, Arguments, Debug, Formatter},
    io::{self, IoSlice, IoSliceMut, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    str::FromStr,
};
use terminal_io::{
    DuplexTerminal, ReadTerminal, Terminal, TerminalColorSupport, TerminalDuplexer, WriteTerminal,
};
use url::Url;
use utf8_io::{ReadStr, ReadStrLayered, Utf8Duplexer, WriteStr};

/// An `InteractiveTextStream` implements `Read` and `Write` as is meant
/// to be used with interactive streams.
///
/// The primary way to construct an `InteractiveTextStream` is to use it as
/// a type in a `StructOpt` struct. Command-line arguments will then
/// be automatically converted into input streams. Currently supported
/// syntaxes include:
///  - Names starting with `connect:` or `accept:`, which are
///    interpreted as socket addresses to connect to or accept from.
///    Socket addresses may contain host:port pairs or, on platforms which
///    support it, filesystem paths to Unix-domain sockets.
///  - "-" is interpreted as the pair (stdin, stdout).
///  - "(...)" runs a command with pipes to and from the child process'
///    (stdin, stdout), on platforms whch support it.
pub struct InteractiveTextStream {
    name: String,
    inner: TextDuplexer<Utf8Duplexer<LayeredDuplexer<TerminalDuplexer<StreamDuplexer>>>>,
}

impl InteractiveTextStream {
    /// Write the given `Pseudonym` to the output stream.
    #[inline]
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        Write::write_all(self, pseudonym.name.as_bytes())
    }

    /// Write the name of the given output stream to the output stream. This is
    /// needed because the name of an `InteractiveTextStream` is not available in
    /// the public API.
    #[inline]
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    /// fixme: dedup some of this with bytestream?
    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        // If we can parse it as a URL, treat it as such.
        if let Ok(url) = Url::parse(s) {
            return Self::from_url(url);
        }

        // Special-case "-" to mean (stdin, stdout).
        if s == "-" {
            return Self::stdin_stdout();
        }

        // Strings beginning with "$(" are commands.
        #[cfg(not(windows))]
        if s.starts_with("$(") {
            return Self::from_child(s);
        }

        // Otherwise try opening it as a path in the filesystem namespace.
        Self::from_path(Path::new(s))
    }

    /// Return an interactive byte stream representing standard input and standard output.
    pub fn stdin_stdout() -> anyhow::Result<Self> {
        let stdin_stdout = InteractiveByteStream::stdin_stdout()?;
        let (name, duplexer) = stdin_stdout.into_parts();
        let duplexer = duplexer.abandon_into_inner().unwrap().into_inner();
        let duplexer = TerminalDuplexer::with_handle(duplexer);
        let duplexer = LayeredDuplexer::new(duplexer);
        let duplexer = Utf8Duplexer::new(duplexer);
        let duplexer = TextDuplexer::with_ansi_color_output(duplexer);
        Ok(Self {
            name,
            inner: duplexer,
        })
    }

    /// Construct a new instance from a URL.
    fn from_url(url: Url) -> anyhow::Result<Self> {
        match url.scheme() {
            "connect" => Self::from_connect_url_str(url),
            "accept" => Self::from_accept_url_str(url),
            scheme @ "http" | scheme @ "https" | scheme @ "file" | scheme @ "data" => {
                Err(anyhow!("non-interactive URL scheme \"{}\"", scheme))
            }
            other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
        }
    }

    fn from_connect_url_str(url: Url) -> anyhow::Result<Self> {
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(anyhow!("connect URL should only contain a socket address"));
        }

        if url.path().is_empty() {
            let port = match url.port() {
                Some(port) => port,
                None => return Err(anyhow!("TCP connect URL should have a port")),
            };
            let host_str = match url.host_str() {
                Some(host_str) => host_str,
                None => return Err(anyhow!("TCP connect URL should have a host")),
            };

            let duplexer = TcpStream::connect(format!("{}:{}", host_str, port))?;
            let duplexer = StreamDuplexer::tcp_stream(duplexer);
            let duplexer = TerminalDuplexer::generic(duplexer);
            let duplexer = LayeredDuplexer::new(duplexer);
            let duplexer = Utf8Duplexer::new(duplexer);
            let duplexer = TextDuplexer::new(duplexer);

            return Ok(Self {
                name: url.to_string(),
                inner: duplexer,
            });
        }

        #[cfg(unix)]
        {
            if url.port().is_some() || url.host_str().is_some() {
                return Err(anyhow!(
                    "Unix-domain connect URL should only contain a path"
                ));
            }

            let duplexer = UnixStream::connect(url.path())?;
            let duplexer = StreamDuplexer::unix_stream(duplexer);
            let duplexer = TerminalDuplexer::generic(duplexer);
            let duplexer = LayeredDuplexer::new(duplexer);
            let duplexer = Utf8Duplexer::new(duplexer);
            let duplexer = TextDuplexer::new(duplexer);

            return Ok(Self {
                name: url.to_string(),
                inner: duplexer,
            });
        }

        #[cfg(windows)]
        return Err(anyhow!("Unsupported connect URL: {}", url));
    }

    fn from_accept_url_str(url: Url) -> anyhow::Result<Self> {
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(anyhow!("accept URL should only contain a socket address"));
        }

        if url.path().is_empty() {
            let port = match url.port() {
                Some(port) => port,
                None => return Err(anyhow!("accept URL should have a port")),
            };
            let host_str = match url.host_str() {
                Some(host_str) => host_str,
                None => return Err(anyhow!("accept URL should have a host")),
            };

            let listener = TcpListener::bind(format!("{}:{}", host_str, port))?;

            let (duplexer, addr) = listener.accept()?;
            let duplexer = StreamDuplexer::tcp_stream(duplexer);
            let duplexer = TerminalDuplexer::generic(duplexer);
            let duplexer = LayeredDuplexer::new(duplexer);
            let duplexer = Utf8Duplexer::new(duplexer);
            let duplexer = TextDuplexer::new(duplexer);

            return Ok(Self {
                name: format!("accept://{}", addr),
                inner: duplexer,
            });
        }

        #[cfg(unix)]
        {
            if url.port().is_some() || url.host_str().is_some() {
                return Err(anyhow!(
                    "Unix-domain connect URL should only contain a path"
                ));
            }

            let listener = UnixListener::bind(url.path())?;

            let (duplexer, addr) = listener.accept()?;
            let duplexer = StreamDuplexer::unix_stream(duplexer);
            let duplexer = TerminalDuplexer::generic(duplexer);
            let duplexer = LayeredDuplexer::new(duplexer);
            let duplexer = Utf8Duplexer::new(duplexer);
            let duplexer = TextDuplexer::new(duplexer);

            let name = path_to_name("accept", addr.as_pathname().unwrap())?;

            return Ok(Self {
                name,
                inner: duplexer,
            });
        }

        #[cfg(windows)]
        return Err(anyhow!("Unsupported connect URL: {}", url));
    }

    /// Construct a new instance from a plain filesystem path.
    #[cfg(not(windows))]
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let name = path_to_name("file", path)?;
        let duplexer = CharDevice::open(path)?;
        let duplexer = StreamDuplexer::char_device(duplexer);
        let duplexer = TerminalDuplexer::with_handle(duplexer);
        let duplexer = LayeredDuplexer::new(duplexer);
        let duplexer = Utf8Duplexer::new(duplexer);
        let duplexer = TextDuplexer::new(duplexer);
        Ok(Self {
            name,
            inner: duplexer,
        })
    }

    /// Construct a new instance from a plain filesystem path.
    #[cfg(windows)]
    fn from_path(_path: &Path) -> anyhow::Result<Self> {
        Err(anyhow!(
            "interactive filesystem paths not supported on Windows yet"
        ))
    }

    #[cfg(not(windows))]
    fn from_child(s: &str) -> anyhow::Result<Self> {
        assert!(s.starts_with("$("));
        if !s.ends_with(')') {
            return Err(anyhow!("child string must end in ')'"));
        }
        let words = shell_words::split(&s[2..s.len() - 1])?;
        let (first, rest) = words
            .split_first()
            .ok_or_else(|| anyhow!("child stream specified with '(...)' must contain a command"))?;
        let mut command = std::process::Command::new(first);
        command.args(rest);
        let duplexer = StreamDuplexer::duplex_with_command(command)?;
        let duplexer = TerminalDuplexer::generic(duplexer);
        let duplexer = LayeredDuplexer::new(duplexer);
        let duplexer = Utf8Duplexer::new(duplexer);
        let duplexer = TextDuplexer::new(duplexer);
        Ok(Self {
            name: s.to_owned(),
            inner: duplexer,
        })
    }
}

/// Implement `FromStr` so that `structopt` can parse `InteractiveTextStream`
/// arguments automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl FromStr for InteractiveTextStream {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

impl ReadLayered for InteractiveTextStream {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        self.inner.read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        self.inner.read_vectored_with_status(bufs)
    }
}

impl Read for InteractiveTextStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.inner.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.inner.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)
    }
}

impl WriteLayered for InteractiveTextStream {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.inner.close()
    }
}

impl WriteStr for InteractiveTextStream {
    #[inline]
    fn write_str(&mut self, buf: &str) -> io::Result<()> {
        self.inner.write_str(buf)
    }
}

impl Write for InteractiveTextStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.inner.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.inner.write_fmt(fmt)
    }
}

impl Terminal for InteractiveTextStream {}

impl ReadTerminal for InteractiveTextStream {
    #[inline]
    fn is_line_by_line(&self) -> bool {
        self.inner.is_line_by_line()
    }

    #[inline]
    fn is_input_terminal(&self) -> bool {
        self.inner.is_input_terminal()
    }
}

impl WriteTerminal for InteractiveTextStream {
    #[inline]
    fn color_support(&self) -> TerminalColorSupport {
        self.inner.color_support()
    }

    #[inline]
    fn color_preference(&self) -> bool {
        self.inner.color_preference()
    }

    #[inline]
    fn is_output_terminal(&self) -> bool {
        self.inner.is_output_terminal()
    }
}

impl DuplexTerminal for InteractiveTextStream {}

impl Duplex for InteractiveTextStream {}

impl Bufferable for InteractiveTextStream {
    #[inline]
    fn abandon(&mut self) {
        self.inner.abandon()
    }
}

impl ReadStr for InteractiveTextStream {
    #[inline]
    fn read_str(&mut self, buf: &mut str) -> io::Result<usize> {
        self.inner.read_str(buf)
    }

    #[inline]
    fn read_exact_str(&mut self, buf: &mut str) -> io::Result<()> {
        self.inner.read_exact_str(buf)
    }
}

impl ReadStrLayered for InteractiveTextStream {
    #[inline]
    fn read_str_with_status(&mut self, buf: &mut str) -> io::Result<(usize, Status)> {
        self.inner.read_str_with_status(buf)
    }

    #[inline]
    fn read_exact_str_using_status(&mut self, buf: &mut str) -> io::Result<Status> {
        self.inner.read_exact_str_using_status(buf)
    }
}

impl Debug for InteractiveTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InteractiveTextStream");
        b.field("inner", &self.inner);
        b.finish()
    }
}
