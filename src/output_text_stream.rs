#[cfg(unix)]
use crate::summon_bat::summon_bat;
use crate::{
    lazy_output::FromLazyOutput,
    open_output::{open_output, Output},
    Pseudonym, Type,
};
use basic_text::{TextStr, TextWriter, WriteText};
use clap::TryFromOsArg;
use io_streams::StreamWriter;
use layered_io::{Bufferable, LayeredWriter, WriteLayered};
#[cfg(all(not(unix), not(windows)))]
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::{
    ffi::{OsStr, OsString},
    fmt::{self, Arguments, Debug, Formatter},
    io::{self, IoSlice, Write},
    process::{exit, Child},
};
use terminal_io::{Terminal, TerminalColorSupport, TerminalWriter, WriteTerminal};
#[cfg(not(windows))]
use unsafe_io::AsUnsafeHandle;
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
    name: String,
    writer: TextWriter<Utf8Writer<LayeredWriter<TerminalWriter<StreamWriter>>>>,
    type_: Type,
    helper_child: Option<(Child, StreamWriter)>,
}

impl OutputTextStream {
    /// Write the given `Pseudonym` to the output stream.
    #[inline]
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        Write::write_all(self, pseudonym.name.as_bytes())
    }

    /// Write the name of the given output stream to the output stream. This is
    /// needed because the name of an `OutputTextStream` is not available in
    /// the public API.
    #[inline]
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    /// If the output stream metadata implies a particular media type, also
    /// known as MIME type, return it. Otherwise default to
    /// "text/plain; charset=utf-8".
    #[inline]
    pub fn type_(&self) -> &Type {
        &self.type_
    }

    fn from_output(output: Output) -> Self {
        #[cfg(unix)]
        let is_stdout = output.writer.eq_handle(&std::io::stdout());
        let terminal = TerminalWriter::with_handle(output.writer);
        #[cfg(unix)]
        let is_terminal = terminal.is_output_terminal();
        #[cfg(unix)]
        let color_support = terminal.color_support();
        #[cfg(unix)]
        let color_preference = terminal.color_preference();

        #[cfg(unix)]
        if is_terminal && is_stdout {
            let stdout_helper_child = summon_bat(&terminal, &output.type_);

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
                    type_: output.type_,
                    helper_child: Some((stdout_helper_child, terminal.into_inner())),
                };
            }
        }

        let writer = LayeredWriter::new(terminal);
        let writer = Utf8Writer::new(writer);
        let writer = TextWriter::with_ansi_color_output(writer);
        let type_ = output.type_.merge(Type::text());
        Self {
            name: output.name,
            writer,
            type_,
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
    fn try_from_os_str_arg(os: &OsStr) -> anyhow::Result<Self> {
        open_output(os, Type::text()).map(Self::from_output)
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
                exit(libc::EXIT_FAILURE);
            }

            match helper_child.0.wait() {
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

impl FromLazyOutput for OutputTextStream {
    type Err = anyhow::Error;

    fn from_lazy_output(name: OsString, type_: Type) -> Result<Self, anyhow::Error> {
        open_output(&name, type_).map(Self::from_output)
    }
}

impl Debug for OutputTextStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("OutputTextStream");
        b.field("type_", &self.type_);
        b.finish()
    }
}
