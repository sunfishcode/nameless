use crate::{path_to_name::path_to_name, Mime, Pseudonym, Type};
use anyhow::anyhow;
use data_url::DataUrl;
use flate2::read::GzDecoder;
use io_ext::{ReadExt, Status};
use io_ext_adapters::StdReader;
use raw_stdio::RawStdin;
use std::{
    fmt::{self, Debug, Formatter},
    fs::File,
    io::{self, Cursor, IoSliceMut},
    path::Path,
    str::FromStr,
};
use url::Url;

/// An input stream for binary input.
///
/// An `InputByteStream` implements `Read` so it supports `read`,
/// `read_to_end`, `read_to_str`, etc. and can be used anywhere a
/// `Read`-implementing object is needed.
///
/// `InputByteStream` is unbuffered (even when it is stdin), so wrapping
/// it in a [`std::io::BufReader`] is recommended for performance and
/// ease of use.
///
/// The primary way to construct an `InputByteStream` is to use it as
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
pub struct InputByteStream {
    name: String,
    reader: Box<dyn ReadExt>,
    type_: Type,
    initial_size: Option<u64>,
}

impl InputByteStream {
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
    /// source is concurrently modified.
    pub fn initial_size(&self) -> Option<u64> {
        self.initial_size
    }

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

        // Special-case "-" to mean stdin.
        if s == "-" {
            return Self::stdin();
        }

        // Strings beginning with "$(" are commands.
        #[cfg(not(windows))]
        if s.starts_with("$(") {
            return Self::from_child(s);
        }

        // Otherwise try opening it as a path in the filesystem namespace.
        Self::from_path(Path::new(s))
    }

    pub(crate) fn from_reader(
        name: String,
        reader: Box<dyn ReadExt>,
        type_: Type,
        initial_size: Option<u64>,
    ) -> Self {
        Self {
            name,
            reader,
            type_,
            initial_size,
        }
    }

    /// Return an input byte stream representing standard input.
    pub fn stdin() -> anyhow::Result<Self> {
        Ok(Self {
            name: "-".to_owned(),
            reader: Box::new(
                RawStdin::new().ok_or_else(|| anyhow!("attempted to open stdin multiple times"))?,
            ),
            type_: Type::unknown(),
            initial_size: None,
        })
    }

    /// Construct a new instance from a URL.
    fn from_url(url: Url) -> anyhow::Result<Self> {
        match url.scheme() {
            "http" | "https" => Self::from_http_url_str(url.as_str()),
            "data" => Self::from_data_url_str(url.as_str()),
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
            other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
        }
    }

    /// Construct a new instance from an `http:` or `https:` URL.
    fn from_http_url_str(http_url_str: &str) -> anyhow::Result<Self> {
        // TODO: Set any headers, like "Accept"?
        let response = ureq::get(http_url_str).call();

        if !response.ok() {
            return Err(anyhow!(
                "HTTP error fetching {}: {}",
                http_url_str,
                response.status_line()
            ));
        }

        let initial_size = Some(
            response
                .header("Content-Length")
                .ok_or_else(|| anyhow!("invalid Content-Length header"))?
                .parse()?,
        );
        let content_type = response.content_type();
        let type_ = Type::from_mime(Mime::from_str(content_type)?);

        Ok(Self {
            name: http_url_str.to_owned(),
            type_,
            reader: Box::new(StdReader::generic(response.into_reader())),
            initial_size,
        })
    }

    /// Construct a new instance from a `data:` URL.
    fn from_data_url_str(data_url_str: &str) -> anyhow::Result<Self> {
        // TODO: `DataUrl` should really implement `std::error::Error`.
        let data_url = DataUrl::process(data_url_str)
            .map_err(|e| anyhow!("invalid data URL syntax: {:?}", e))?;
        // TODO: `DataUrl` should really really implement `std::error::Error`.
        let (body, fragment) = data_url
            .decode_to_vec()
            .map_err(|_| anyhow!("invalid base64 encoding"))?;

        if fragment.is_some() {
            return Err(anyhow!("data urls with fragments are unsupported"));
        }

        // Awkwardly convert from `data_url::Mime` to `mime::Mime`.
        // TODO: Consider submitting patches to `data_url` to streamline this.
        let type_ = Type::from_mime(Mime::from_str(&data_url.mime_type().to_string()).unwrap());

        Ok(Self {
            name: data_url_str.to_owned(),
            reader: Box::new(StdReader::generic(Cursor::new(body))),
            type_,
            initial_size: Some(data_url_str.len() as u64),
        })
    }

    /// Construct a new instance from a plain filesystem path.
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let name = path_to_name("file", path)?;
        // TODO: Should we have our own error type?
        let file = File::open(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
        if path.extension() == Some(Path::new("gz").as_os_str()) {
            // TODO: We shouldn't really need to allocate a `PathBuf` here.
            let path = path.with_extension("");
            let type_ = Type::from_extension(path.extension());
            let initial_size = None;
            Ok(Self {
                name,
                reader: Box::new(StdReader::generic(GzDecoder::new(file))),
                type_,
                initial_size,
            })
        } else {
            let type_ = Type::from_extension(path.extension());
            let initial_size = Some(file.metadata()?.len());
            Ok(Self {
                name,
                reader: Box::new(StdReader::new(file)),
                type_,
                initial_size,
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
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?;
        Ok(Self {
            name: s.to_owned(),
            reader: Box::new(StdReader::new(child.stdout.unwrap())),
            type_: Type::unknown(),
            initial_size: None,
        })
    }
}

/// Implement `FromStr` so that `structopt` can parse `InputByteStream`
/// arguments automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. Two potential concerns:
///  - This uses `str` so it only handles well-formed Unicode paths.
///  - Opening resources from strings depends on ambient authorities.
#[doc(hidden)]
impl FromStr for InputByteStream {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
}

impl ReadExt for InputByteStream {
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

impl io::Read for InputByteStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.reader.read_vectored(bufs)
    }

    #[cfg(feature = "nightly")]
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

impl Debug for InputByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Don't print the name here, as that's an implementation detail.
        let mut b = f.debug_struct("InputByteStream");
        b.field("type_", &self.type_);
        b.field("initial_size", &self.initial_size);
        b.finish()
    }
}

#[test]
fn data_url_plain() {
    use std::io::Read;
    let mut s = String::new();
    InputByteStream::from_str("data:,Hello%2C%20World!")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "Hello, World!");
}

#[test]
fn data_url_base64() {
    use std::io::Read;
    let mut s = String::new();
    InputByteStream::from_str("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "Hello, World!");
}
