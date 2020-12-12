#[cfg(unix)]
use crate::summon_bat::summon_bat;
use crate::{path_to_name::path_to_name, OutputByteStream, Pseudonym, Type};
use anyhow::anyhow;
use flate2::{write::GzEncoder, Compression};
use io_ext::{default_flush, Status, WriteExt};
use io_ext_adapters::StdWriter;
use raw_stdio::RawStdout;
#[cfg(all(not(unix), not(windows)))]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::{
    fmt::{self, Arguments, Debug, Formatter},
    fs::File,
    io::{self, IoSlice},
    path::Path,
    process::{exit, Child},
    str::FromStr,
};
use terminal_support::{detect_terminal_color_support, Terminal, TerminalColorSupport};
use text_streams::TextWriter;
use url::Url;

/// An output stream for plain text output.
///
/// An `OutputTextStream` implements `Write` so it supports `write`,
/// `write_all`, etc. and can be used anywhere a `Write`-implementing
/// object is needed.
///
/// `OutputTextStream` is unbuffered (even when it is stdout), so wrapping
/// it in a [`std::io::BufWriter`] or [`std::io::LineWriter`] is
/// recommended for performance.
///
/// The primary way to construct an `OutputTextStream` is to use it as
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
/// Programs using `OutputTextStream` as an argument should avoid using
/// `std::io::stdout`, `std::println`, or anything else which uses standard
/// output implicitly.
pub struct OutputTextStream {
    inner: OutputByteStream,
    stdout_helper_child: Option<(Child, RawStdout)>,
    color_support: TerminalColorSupport,
}

impl OutputTextStream {
    /// Write the given `Pseudonym` to the output stream.
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        self.inner.write_pseudonym(pseudonym)
    }

    /// Write the name of the given output stream to the output stream. This is
    /// needed because the name of an `OutputTextStream` is not available in
    /// the public API.
    pub fn pseudonym(&self) -> Pseudonym {
        self.inner.pseudonym()
    }

    /// If the output stream metadata implies a particular media type, also
    /// known as MIME type, return it. Otherwise default to
    /// "text/plain; charset=utf-8".
    pub fn type_(&self) -> &Type {
        self.inner.type_()
    }

    /// FIXME: dedup some of this with bytestream?
    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        // If we can parse it as a URL, treat it as such.
        if let Ok(url) = Url::parse(s) {
            return Self::from_url(url);
        }

        // Special-case "-" to mean stdout.
        if s == "-" {
            return Self::stdout(Type::text());
        }

        // Strings beginning with "$(" are commands.
        #[cfg(not(windows))]
        if s.starts_with("$(") {
            return Self::from_child(s);
        }

        // Otherwise try opening it as a path in the filesystem namespace.
        Self::from_path(Path::new(s))
    }

    /// Return an output byte stream representing standard output.
    pub fn stdout(type_: Type) -> anyhow::Result<Self> {
        let stdout =
            RawStdout::new().ok_or_else(|| anyhow!("attempted to open stdout multiple times"))?;

        #[cfg(unix)]
        let color_support = {
            let (_isatty, color_support, stdout_helper_child) = summon_bat(&stdout, &type_)?;

            if let Some(mut stdout_helper_child) = stdout_helper_child {
                let output = StdWriter::new(stdout_helper_child.stdin.take().unwrap());
                let output = TextWriter::with_ansi_color(
                    output,
                    color_support != TerminalColorSupport::Monochrome,
                );

                return Ok(Self {
                    inner: OutputByteStream::from_writer("-".to_owned(), Box::new(output), type_),
                    stdout_helper_child: Some((stdout_helper_child, stdout)),
                    color_support,
                });
            }

            color_support
        };

        #[cfg(all(not(unix), not(windows)))]
        let (_isatty, color_support) =
            detect_terminal_color_support(&std::mem::ManuallyDrop::new(unsafe {
                File::from_raw_fd(stdout.as_raw_fd())
            }));

        #[cfg(windows)]
        let (_isatty, color_support) =
            detect_terminal_color_support(&std::mem::ManuallyDrop::new(unsafe {
                File::from_raw_handle(stdout.as_raw_handle())
            }));

        let stdout =
            TextWriter::with_ansi_color(stdout, color_support != TerminalColorSupport::Monochrome);

        Ok(Self {
            inner: OutputByteStream::from_writer("-".to_owned(), Box::new(stdout), type_),
            stdout_helper_child: None,
            color_support,
        })
    }

    /// Construct a new instance from a URL.
    fn from_url(url: Url) -> anyhow::Result<Self> {
        match url.scheme() {
            // TODO: POST the data to HTTP? But the `Write` trait makes this
            // tricky because there's no hook for closing and finishing the
            // stream. `Drop` can't fail.
            "http" | "https" => Err(anyhow!("output to HTTP not supported yet")),
            "file" => {
                if !url.username().is_empty()
                    || url.password().is_some()
                    || url.has_host()
                    || url.port().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                {
                    return Err(anyhow!("file URL should only contain a path"));
                }
                // TODO: https://docs.rs/url/latest/url/struct.Url.html#method.to_file_path
                // is ambiguous about how it can fail. What is `Path::new_opt`?
                Self::from_path(
                    &url.to_file_path()
                        .map_err(|_: ()| anyhow!("unknown file URL weirdness"))?,
                )
            }
            "data" => Err(anyhow!("output to data URL isn't possible")),
            other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
        }
    }

    /// Construct a new instance from a plain filesystem path.
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let name = path_to_name("file", path)?;
        let file = File::create(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
        if path.extension() == Some(Path::new("gz").as_os_str()) {
            // TODO: We shouldn't really need to allocate a `PathBuf` here.
            let path = path.with_extension("");
            let type_ = Type::from_extension(path.extension());
            // 6 is the default gzip compression level.
            let writer = StdWriter::new(GzEncoder::new(file, Compression::new(6)));
            let writer = TextWriter::new(writer);
            Ok(Self {
                inner: OutputByteStream::from_writer(name, Box::new(writer), type_),
                stdout_helper_child: None,
                color_support: TerminalColorSupport::default(),
            })
        } else {
            let type_ = Type::from_extension(path.extension());
            // Even though we opened this from the filesystem, it might be a
            // character device and might have color support.
            let (_isatty, color_support) = detect_terminal_color_support(&file);
            let writer = StdWriter::new(file);
            let writer = TextWriter::with_ansi_color(
                writer,
                color_support != TerminalColorSupport::Monochrome,
            );
            Ok(Self {
                inner: OutputByteStream::from_writer(name, Box::new(writer), type_),
                stdout_helper_child: None,
                color_support,
            })
        }
    }

    #[cfg(not(windows))]
    fn from_child(s: &str) -> anyhow::Result<Self> {
        use std::process::{Command, Stdio};
        assert!(s.starts_with("$("));
        if !s.ends_with(')') {
            return Err(anyhow!("child string must end in ')'"));
        }
        let words = shell_words::split(&s[2..s.len() - 1])?;
        let (first, rest) = words
            .split_first()
            .ok_or_else(|| anyhow!("child stream specified with '(...)' must contain a command"))?;
        let child = Command::new(first)
            .args(rest)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;
        let writer = StdWriter::new(child.stdin.unwrap());
        let writer = TextWriter::new(writer);
        Ok(Self {
            inner: OutputByteStream::from_writer(s.to_owned(), Box::new(writer), Type::unknown()),
            stdout_helper_child: None,
            color_support: TerminalColorSupport::default(),
        })
    }
}

/// Implement `From<&OsStr>` so that `structopt` can parse `OutputTextStream`
/// objects automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl FromStr for OutputTextStream {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

impl WriteExt for OutputTextStream {
    #[inline]
    fn flush_with_status(&mut self, status: Status) -> io::Result<()> {
        self.inner.flush_with_status(status)?;

        if let Status::End = status {
            if let Some((mut stdout_helper_child, _raw_stdout)) = self.stdout_helper_child.take() {
                // Close standard output, prompting the child process to exit.
                self.inner = OutputByteStream::from_writer(
                    "-".to_owned(),
                    Box::new(Vec::new()),
                    self.inner.type_().clone(),
                );

                stdout_helper_child.wait()?;
            }
        }

        Ok(())
    }

    #[inline]
    fn abandon(&mut self) {
        self.inner.abandon()
    }

    #[inline]
    fn write_str(&mut self, buf: &str) -> io::Result<()> {
        self.inner.write_str(buf)
    }
}

impl io::Write for OutputTextStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        default_flush(self)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.inner.write_all_vectored(bufs)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.inner.write_fmt(fmt)
    }
}

impl Terminal for OutputTextStream {
    fn color_support(&self) -> TerminalColorSupport {
        self.color_support
    }
}

impl Drop for OutputTextStream {
    fn drop(&mut self) {
        if let Some((mut stdout_helper_child, _raw_stdout)) = self.stdout_helper_child.take() {
            // Wait for the child. We can't return `Err` from a `drop` function,
            // so just print a message and return. Callers should use
            // `flush_with_status(Status::End)` to declare the end of the stream
            // if they wish to avoid these errors.

            // Close standard output, prompting the child process to exit.
            self.inner = OutputByteStream::from_writer(
                "-".to_owned(),
                Box::new(Vec::new()),
                self.inner.type_().clone(),
            );

            match stdout_helper_child.wait() {
                Ok(status) => {
                    if !status.success() {
                        eprintln!(
                            "Output formatting process exited with non-success exit status: {:?}",
                            status
                        );
                        exit(libc::EXIT_FAILURE);
                    }
                }

                Err(e) => {
                    eprintln!("Unable to wait for output formatting process: {:?}", e);
                    exit(libc::EXIT_FAILURE);
                }
            }
        }
    }
}

impl Debug for OutputTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("OutputTextStream");
        b.field("inner", &self.inner);
        b.finish()
    }
}
