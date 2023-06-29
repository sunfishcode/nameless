use crate::lazy_output::FromLazyOutput;
use crate::open_output::{open_output, Output};
#[cfg(unix)]
use crate::summon_bat::summon_bat;
use crate::{MediaType, Pseudonym};
use basic_text::{TextStr, TextWriter, WriteText};
use clap::{AmbientAuthority, TryFromOsArg};
#[cfg(not(windows))]
use io_extras::os::rustix::AsRawFd;
use io_streams::StreamWriter;
use layered_io::{Bufferable, LayeredWriter, WriteLayered};
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Arguments, Debug, Formatter};
use std::io::{self, IoSlice, Write};
use std::process::{exit, Child};
use terminal_io::{Terminal, TerminalColorSupport, TerminalWriter, WriteTerminal};
use utf8_io::{Utf8Writer, WriteStr};

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
/// a type in a `kommand` argument or a `clap_derive` struct. Command-line
/// arguments will then be automatically converted into output streams.
/// Currently supported syntaxes include:
///  - Names starting with `file:` are interpreted as local filesystem URLs
///    providing paths to files to open.
///  - "-" is interpreted as standard output.
///  - "(...)" runs a command with a pipe to the child process' stdin, on
///    platforms whch support it.
///  - Names which don't parse as URLs are interpreted as plain local
///    filesystem paths. To force a string to be interpreted as a plain local
///    path, arrange for it to begin with `./` or `/`.
///
/// Programs using `OutputTextStream` as an argument should avoid using
/// `std::io::stdout`, `std::println`, or anything else which uses standard
/// output implicitly.
pub struct OutputTextStream {
    name: String,
    writer: TextWriter<Utf8Writer<LayeredWriter<TerminalWriter<StreamWriter>>>>,
    media_type: MediaType,
    helper_child: Option<(Child, StreamWriter)>,
}

impl OutputTextStream {
    /// Write the given `Pseudonym` to the output stream.
    #[inline]
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        Write::write_all(self, pseudonym.name.as_bytes())
    }

    /// Return a `Pseudonym` which encapsulates this stream's name (typically
    /// its filesystem path or its URL). This allows it to be written to an
    /// `OutputByteStream` while otherwise remaining entirely opaque.
    #[inline]
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    /// If the output stream metadata implies a particular media type, also
    /// known as MIME type, return it. Otherwise default to
    /// "text/plain; charset=utf-8".
    #[inline]
    pub fn media_type(&self) -> &MediaType {
        &self.media_type
    }

    fn from_output(output: Output) -> Self {
        #[cfg(unix)]
        let is_stdout = output.writer.as_raw_fd() == rustix::stdio::raw_stdout();
        let terminal = TerminalWriter::with_handle(output.writer);
        #[cfg(unix)]
        let is_terminal = terminal.is_output_terminal();
        #[cfg(unix)]
        let color_support = terminal.color_support();
        #[cfg(unix)]
        let color_preference = terminal.color_preference();

        #[cfg(unix)]
        if is_terminal && is_stdout {
            let stdout_helper_child = summon_bat(&terminal, &output.media_type);

            if let Some(mut stdout_helper_child) = stdout_helper_child {
                let writer = StreamWriter::child_stdin(stdout_helper_child.stdin.take().unwrap());
                let writer =
                    TerminalWriter::from(writer, is_terminal, color_support, color_preference);
                let writer = LayeredWriter::new(writer);
                let writer = Utf8Writer::new(writer);
                let writer = TextWriter::with_ansi_color_output(writer);

                return Self {
                    name: output.name,
                    writer,
                    media_type: output.media_type,
                    helper_child: Some((stdout_helper_child, terminal.into_inner())),
                };
            }
        }

        let writer = LayeredWriter::new(terminal);
        let writer = Utf8Writer::new(writer);
        let writer = TextWriter::with_ansi_color_output(writer);
        let media_type = output.media_type.union(MediaType::text());
        Self {
            name: output.name,
            writer,
            media_type,
            helper_child: None,
        }
    }
}

/// Implement `From<&OsStr>` so that `clap_derive` can parse `OutputTextStream`
/// objects automatically.
///
/// This is hidden from the documentation as it opens resources from
/// strings using ambient authorities.
#[doc(hidden)]
impl TryFromOsArg for OutputTextStream {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_os_str_arg(
        os: &OsStr,
        ambient_authority: AmbientAuthority,
    ) -> anyhow::Result<Self> {
        open_output(os, MediaType::text(), ambient_authority).map(Self::from_output)
    }
}

impl WriteLayered for OutputTextStream {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.writer.close()?;

        if let Some(mut helper_child) = self.helper_child.take() {
            helper_child.0.wait()?;
        }

        Ok(())
    }
}

impl WriteStr for OutputTextStream {
    #[inline]
    fn write_str(&mut self, buf: &str) -> io::Result<()> {
        self.writer.write_str(buf)
    }
}

impl Write for OutputTextStream {
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

impl Bufferable for OutputTextStream {
    #[inline]
    fn abandon(&mut self) {
        self.writer.abandon()
    }
}

impl Terminal for OutputTextStream {}

impl WriteTerminal for OutputTextStream {
    #[inline]
    fn color_support(&self) -> TerminalColorSupport {
        self.writer.color_support()
    }

    #[inline]
    fn color_preference(&self) -> bool {
        self.writer.color_preference()
    }

    #[inline]
    fn is_output_terminal(&self) -> bool {
        self.writer.is_output_terminal()
    }
}

impl WriteText for OutputTextStream {
    #[inline]
    fn write_text(&mut self, buf: &TextStr) -> io::Result<()> {
        self.writer.write_text(buf)
    }
}

impl Drop for OutputTextStream {
    fn drop(&mut self) {
        if let Some(mut helper_child) = self.helper_child.take() {
            // Wait for the child. We can't return `Err` from a `drop` function,
            // so just print a message and exit. Callers should use
            // `end()` to declare the end of the stream if they wish to avoid
            // these errors.

            // Close standard output, prompting the child process to exit.
            if let Err(e) = self.writer.close() {
                eprintln!("Output formatting process encountered error: {:?}", e);
                #[cfg(not(windows))]
                exit(rustix::process::EXIT_FAILURE);
                #[cfg(windows)]
                exit(libc::EXIT_FAILURE);
            }

            match helper_child.0.wait() {
                Ok(status) => {
                    if !status.success() {
                        eprintln!(
                            "Output formatting process exited with non-success exit status: {:?}",
                            status
                        );
                        #[cfg(not(windows))]
                        exit(rustix::process::EXIT_FAILURE);
                        #[cfg(windows)]
                        exit(libc::EXIT_FAILURE);
                    }
                }

                Err(e) => {
                    eprintln!("Unable to wait for output formatting process: {:?}", e);
                    #[cfg(not(windows))]
                    exit(rustix::process::EXIT_FAILURE);
                    #[cfg(windows)]
                    exit(libc::EXIT_FAILURE);
                }
            }
        }
    }
}

impl FromLazyOutput for OutputTextStream {
    type Err = anyhow::Error;

    fn from_lazy_output(
        name: OsString,
        media_type: MediaType,
        ambient_authority: AmbientAuthority,
    ) -> Result<Self, anyhow::Error> {
        open_output(&name, media_type, ambient_authority).map(Self::from_output)
    }
}

impl Debug for OutputTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("OutputTextStream");
        b.field("media_type", &self.media_type);
        b.finish()
    }
}
