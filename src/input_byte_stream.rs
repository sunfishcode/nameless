use crate::path_url::path_url;
use crate::Mime;
use anyhow::anyhow;
use data_url::DataUrl;
use flate2::read::GzDecoder;
use std::{
    fmt::{self, Debug, Display, Formatter},
    fs::File,
    io::{self, stdin, Cursor, IoSliceMut, Read},
    path::Path,
    str::FromStr,
};
use url::Url;

/// An `InputByteStream` has `deref`s to `Read` so it supports `read`,
/// `read_to_end`, `read_to_str`, etc. and can be used anywhere a
/// `Read`-implementing object is needed.
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
///  - Names which don't parse as URLs are interpreted as plain local
///    filesystem paths.
pub struct InputByteStream {
    pub(crate) name: String,
    reader: Box<dyn Read>,
    mime: Option<Mime>,
    initial_size: Option<u64>,
}

impl InputByteStream {
    /// If the input stream metadata implies a particular media type, also
    /// known as MIME type, return it. Many input streams know their type,
    /// though some do not. This is strictly based on available metadata, and
    /// not on examining any of the contents of the stream, and there's no
    /// guarantee the contents are valid.
    pub fn mime(&self) -> Option<&Mime> {
        self.mime.as_ref()
    }

    /// Return the initial size of the stream, in bytes. This is strictly based
    /// on available metadata, and not on examining any of the contents of the
    /// stream, and the stream could end up being shorter or longer.
    pub fn initial_size(&self) -> Option<u64> {
        self.initial_size
    }

    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        // If we can parse it as a URL, treat it as such.
        if let Ok(url) = Url::parse(s) {
            return Self::from_url(url);
        }

        // Special-case "-" to mean stdin.
        if s == "-" {
            return Ok(Self::stdin());
        }

        // Otherwise try opening it as a path in the filesystem namespace.
        // FIXME: `percent_decode`
        Self::from_path(Path::new(s))
    }

    fn stdin() -> Self {
        Self {
            name: "-".to_owned(),
            reader: Box::new(stdin()),
            mime: None,
            initial_size: None,
        }
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
                // FIXME: https://docs.rs/url/latest/url/struct.Url.html#method.to_file_path
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
        let mime = Some(Mime::from_str(content_type)?);

        Ok(Self {
            name: http_url_str.to_owned(),
            mime,
            reader: Box::new(response.into_reader()),
            initial_size,
        })
    }

    /// Construct a new instance from a `data:` URL.
    fn from_data_url_str(data_url_str: &str) -> anyhow::Result<Self> {
        // FIXME: `DataUrl` should really implement `std::error::Error`.
        let data_url = DataUrl::process(data_url_str)
            .map_err(|e| anyhow!("invalid data URL syntax: {:?}", e))?;
        // FIXME: `DataUrl` should really really implement `std::error::Error`.
        let (body, fragment) = data_url
            .decode_to_vec()
            .map_err(|_| anyhow!("invalid base64 encoding"))?;

        if fragment.is_some() {
            return Err(anyhow!("data urls with fragments are unsupported"));
        }

        // Awkwardly convert from `data_url::Mime` to `mime::Mime`.
        // TODO: Consider submitting patches to `data_url` to streamline this.
        let mime = Some(Mime::from_str(&data_url.mime_type().to_string()).unwrap());

        Ok(Self {
            name: data_url_str.to_owned(),
            reader: Box::new(Cursor::new(body)),
            mime,
            initial_size: Some(data_url_str.len() as u64),
        })
    }

    /// Construct a new instance from a plain filesystem path.
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        // TODO: Should we have our own error type?
        let file = File::open(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
        let name = path_url(path);
        if path.extension() == Some(Path::new("gz").as_os_str()) {
            // TODO: We shouldn't really need to allocate a `PathBuf` here.
            let path = path.with_extension("");
            let mime = mime_guess::from_path(&path).first();
            let initial_size = None;
            Ok(Self {
                name,
                reader: Box::new(GzDecoder::new(file)),
                mime,
                initial_size,
            })
        } else {
            let mime = mime_guess::from_path(path).first();
            let initial_size = Some(file.metadata()?.len());
            Ok(Self {
                name,
                reader: Box::new(file),
                mime,
                initial_size,
            })
        }
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

/// Implement `Default` so that `structopt` can give `InputByteStream` default
/// values automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. A potential concern:
///  - Opening resources as a default assumes ambient authorities.
#[doc(hidden)]
impl Default for InputByteStream {
    fn default() -> Self {
        Self::stdin()
    }
}

/// Implement `Display` so that `structopt` can give `InputByteStream` default
/// values automatically. For now, hide this from the documentation as it's
/// not clear if we want to commit to this approach. A potential concern:
///  - Opening resources as a default assumes ambient authorities.
#[doc(hidden)]
impl Display for InputByteStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.name, f)
    }
}

impl Read for InputByteStream {
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
        b.field("mime", &self.mime);
        b.field("initial_size", &self.initial_size);
        b.finish()
    }
}

#[test]
fn data_url_plain() {
    let mut s = String::new();
    InputByteStream::from_str("data:,Hello%2C%20World!")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "Hello, World!");
}

#[test]
fn data_url_base64() {
    let mut s = String::new();
    InputByteStream::from_str("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, "Hello, World!");
}
