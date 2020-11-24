use crate::{stdin_stdout::StdinStdout, Pseudonym};
use anyhow::anyhow;
use io_ext::{
    default_read, default_read_exact, default_read_to_end, default_read_to_string,
    default_read_vectored, ReadExt, ReadWriteExt, Status, WriteExt,
};
use io_ext_adapters::StdReaderWriter;
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::{
    fmt::{self, Arguments, Debug, Formatter},
    io::{self, IoSlice, IoSliceMut},
    net::{TcpListener, TcpStream},
    path::Path,
    str::FromStr,
};
use url::Url;
#[cfg(not(windows))]
use {crate::path_to_name::path_to_name, std::fs::OpenOptions};

/// An `InteractiveByteStream` implements `Read` and `Write` as is meant
/// to be used with interactive streams.
///
/// The primary way to construct an `InteractiveByteStream` is to use it as
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
pub struct InteractiveByteStream {
    name: String,
    reader_writer: Box<dyn ReadWriteExt>,
}

impl InteractiveByteStream {
    /// Return a `Pseudonym` which encapsulates this stream's name (typically
    /// its filesystem path or its URL). This allows it to be written to an
    /// `OutputByteStream` while otherwise remaining entirely opaque.
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

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

    pub(crate) fn from_reader_writer(name: String, reader_writer: Box<dyn ReadWriteExt>) -> Self {
        Self {
            name,
            reader_writer,
        }
    }

    /// Return an interactive byte stream representing standard input and standard output.
    pub fn stdin_stdout() -> anyhow::Result<Self> {
        Ok(Self {
            name: "-".to_owned(),
            reader_writer: Box::new(
                StdinStdout::new()
                    .ok_or_else(|| anyhow!("attempted to open stdin or stdout multiple times"))?,
            ),
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

            let stream = TcpStream::connect(format!("{}:{}", host_str, port))?;
            let stream = StdReaderWriter::new(stream);

            return Ok(Self {
                name: url.to_string(),
                reader_writer: Box::new(stream),
            });
        }

        #[cfg(not(windows))]
        {
            if url.port().is_some() || url.host_str().is_some() {
                return Err(anyhow!(
                    "Unix-domain connect URL should only contain a path"
                ));
            }

            let stream = UnixStream::connect(url.path())?;
            let stream = StdReaderWriter::new(stream);

            return Ok(Self {
                name: url.to_string(),
                reader_writer: Box::new(stream),
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

            let (stream, addr) = listener.accept()?;
            let stream = StdReaderWriter::new(stream);

            return Ok(Self {
                name: format!("accept://{}", addr),
                reader_writer: Box::new(stream),
            });
        }

        #[cfg(not(windows))]
        {
            if url.port().is_some() || url.host_str().is_some() {
                return Err(anyhow!(
                    "Unix-domain connect URL should only contain a path"
                ));
            }

            let listener = UnixListener::bind(url.path())?;

            let (stream, addr) = listener.accept()?;
            let stream = StdReaderWriter::new(stream);

            let name = path_to_name("accept", addr.as_pathname().unwrap())?;

            return Ok(Self {
                name,
                reader_writer: Box::new(stream),
            });
        }

        #[cfg(windows)]
        return Err(anyhow!("Unsupported connect URL: {}", url));
    }

    /// Construct a new instance from a plain filesystem path.
    #[cfg(not(windows))]
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        use std::os::unix::fs::FileTypeExt;

        let name = path_to_name("file", path)?;
        // TODO: Should we have our own error type?
        let mut options = OpenOptions::new();
        let file = options
            .read(true)
            .write(true)
            .open(path)
            .map_err(|err| anyhow!("{}: {}", path.display(), err))?;
        let metadata = file.metadata()?;
        if !metadata.file_type().is_char_device() {
            return Err(anyhow!(
                "path to interactive channel must be a character device"
            ));
        }
        Ok(Self {
            name,
            reader_writer: Box::new(StdReaderWriter::new(file)),
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
        Ok(Self {
            name: s.to_owned(),
            reader_writer: Box::new(crate::command_stdin_stdout::CommandStdinStdout::new(
                command,
            )),
        })
    }
}

/// Implement `FromStr` so that `structopt` can parse `InteractiveByteStream`
/// arguments automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl FromStr for InteractiveByteStream {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

impl ReadExt for InteractiveByteStream {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        self.reader_writer.read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        self.reader_writer.read_vectored_with_status(bufs)
    }
}

impl io::Read for InteractiveByteStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        default_read(self, buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        default_read_vectored(self, bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.reader_writer.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        default_read_to_end(self, buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        default_read_to_string(self, buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        default_read_exact(self, buf)
    }
}

impl WriteExt for InteractiveByteStream {
    #[inline]
    fn flush_with_status(&mut self, status: Status) -> io::Result<()> {
        self.reader_writer.flush_with_status(status)
    }

    #[inline]
    fn abandon(&mut self) {
        self.reader_writer.abandon()
    }

    #[inline]
    fn write_str(&mut self, buf: &str) -> io::Result<()> {
        self.reader_writer.write_str(buf)
    }
}

impl io::Write for InteractiveByteStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.reader_writer.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.reader_writer.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.reader_writer.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.reader_writer.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.reader_writer.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.reader_writer.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.reader_writer.write_fmt(fmt)
    }
}

impl Debug for InteractiveByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InteractiveByteStream");
        b.finish()
    }
}
