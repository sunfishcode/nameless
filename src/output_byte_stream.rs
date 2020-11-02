use crate::path_url::path_url;
use crate::{Pseudonym, Mime};
use anyhow::anyhow;
use flate2::{write::GzEncoder, Compression};
use std::{
    fmt::{self, Arguments, Debug, Display, Formatter},
    fs::File,
    io::{self, stdout, IoSlice, Write},
    path::Path,
    str::FromStr,
};
use url::Url;

/// An `OutputByteStream` has `deref`s to `Write` so it supports `write`,
/// `write_all`, etc. and can be used anywhere a `Write`-implementing
/// object is needed.
///
/// The primary way to construct an `OutputByteStream` is to use it as
/// a type in a `StructOpt` struct. Command-line arguments will then
/// be automatically converted into output streams. Currently supported
/// syntaxes include:
///  - Names starting with `file:` are interpreted as local filesystem
///    URLs providing paths to files to open.
///  - Names which don't parse as URLs are interpreted as plain local
///    filesystem paths.
pub struct OutputByteStream {
    name: String,
    writer: Box<dyn Write>,
    mime: Option<Mime>,
}

impl OutputByteStream {
    /// Write the given `Pseudonym` to the output stream.
    pub fn write_pseudonym(&mut self, pseudonym: &Pseudonym) -> io::Result<()> {
        self.write_all(pseudonym.name.as_bytes())
    }

    /// Write the name of the given output stream to the output stream. This is
    /// needed because the name of an `OutputByteStream` is not available in
    /// the public API.
    pub fn pseudonym(&self) -> Pseudonym {
        Pseudonym::new(self.name.clone())
    }

    /// Return the given stream or stdout.
    pub fn default_to_stdout(maybe_me: Option<Self>) -> Self {
        maybe_me.unwrap_or_else(Self::stdout)
    }

    /// If the output stream metadata implies a particular media type, also
    /// known as MIME type, return it. Some output streams know their type,
    /// though many do not.
    pub fn mime(&self) -> Option<&Mime> {
        self.mime.as_ref()
    }

    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        // If we can parse it as a URL, treat it as such.
        if let Ok(url) = Url::parse(s) {
            return Self::from_url(url);
        }

        // Special-case "-" to mean stdout.
        if s == "-" {
            return Ok(Self::stdout());
        }

        // Otherwise try opening it as a path in the filesystem namespace.
        Self::from_path(Path::new(s))
    }

    fn stdout() -> Self {
        Self {
            name: "-".to_owned(),
            writer: Box::new(stdout()),
            mime: None,
        }
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
                // FIXME: https://docs.rs/url/latest/url/struct.Url.html#method.to_file_path
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
        let file = File::create(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
        let name = path_url(path);
        if path.extension() == Some(Path::new("gz").as_os_str()) {
            // TODO: We shouldn't really need to allocate a `PathBuf` here.
            let path = path.with_extension("");
            let mime = mime_guess::from_path(&path).first();
            // 6 is the default gzip compression level.
            Ok(Self {
                name,
                writer: Box::new(GzEncoder::new(file, Compression::new(6))),
                mime,
            })
        } else {
            let mime = mime_guess::from_path(&path).first();
            Ok(Self {
                name,
                writer: Box::new(file),
                mime,
            })
        }
    }
}

/// Implement `From<&OsStr>` so that `structopt` can parse `OutputByteStream`
/// objects automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl FromStr for OutputByteStream {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

/// Implement `Default` so that `structopt` can give `OutputByteStream` default
/// values automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. A potential concern:
///  - Opening resources as a default assumes ambient authorities.
#[doc(hidden)]
impl Default for OutputByteStream {
    fn default() -> Self {
        Self::stdout()
    }
}

/// Implement `Display` so that `structopt` can give `OutputByteStream` default
/// values automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. A potential concern:
///  - Opening resources as a default assumes ambient authorities.
#[doc(hidden)]
impl Display for OutputByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.name, f)
    }
}

impl Write for OutputByteStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.writer.write_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
    fn is_write_vectored(&self) -> bool {
        self.writer.is_write_vectored()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.writer.write_all(buf)
    }

    #[cfg(feature = "nightly")]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        self.writer.write_all_vectored(bufs)
    }

    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.writer.write_fmt(fmt)
    }
}

impl Debug for OutputByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("OutputByteStream");
        b.finish()
    }
}
