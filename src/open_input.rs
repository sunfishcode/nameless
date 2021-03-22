use crate::{path_to_name::path_to_name, MediaType, Mime};
use anyhow::anyhow;
use data_url::DataUrl;
use flate2::read::GzDecoder;
use io_streams::StreamReader;
use std::{convert::TryInto, ffi::OsStr, fs::File, path::Path, str::FromStr};
use url::Url;
#[cfg(feature = "ssh2")]
use {percent_encoding::percent_decode, ssh2::Session, std::net::TcpStream};

pub(crate) struct Input {
    pub(crate) name: String,
    pub(crate) reader: StreamReader,
    pub(crate) media_type: MediaType,
    pub(crate) initial_size: Option<u64>,
}

pub(crate) fn open_input(os: &OsStr) -> anyhow::Result<Input> {
    if let Some(s) = os.to_str() {
        // If we can parse it as a URL, treat it as such.
        if let Ok(url) = Url::parse(s) {
            return open_url(url);
        }

        // Special-case "-" to mean stdin.
        if s == "-" {
            return acquire_stdin();
        }
    }

    #[cfg(not(windows))]
    {
        let lossy = os.to_string_lossy();

        // Strings beginning with "$(" are commands.
        if lossy.starts_with("$(") {
            return spawn_child(os, &lossy);
        }
    }

    // Otherwise try opening it as a path in the filesystem namespace.
    open_path(Path::new(os))
}

fn acquire_stdin() -> anyhow::Result<Input> {
    let reader = StreamReader::stdin()?;
    Ok(Input {
        name: "-".to_owned(),
        reader,
        media_type: MediaType::unknown(),
        initial_size: None,
    })
}

fn open_url(url: Url) -> anyhow::Result<Input> {
    match url.scheme() {
        "http" | "https" => open_http_url_str(url.as_str()),
        "data" => open_data_url_str(url.as_str()),
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
            open_path(
                &url.to_file_path()
                    .map_err(|_: ()| anyhow!("unknown file URL weirdness"))?,
            )
        }
        #[cfg(feature = "ssh2")]
        "scp" => open_scp_url(&url),
        other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
    }
}

fn open_http_url_str(http_url_str: &str) -> anyhow::Result<Input> {
    // TODO: Set any headers, like "Accept"?
    let response = ureq::get(http_url_str)
        .call()
        .map_err(|e| anyhow!("HTTP error fetching {}: {}", http_url_str, e))?;

    let initial_size = Some(
        response
            .header("Content-Length")
            .ok_or_else(|| anyhow!("invalid Content-Length header"))?
            .parse()?,
    );
    let media_type = response.content_type();
    let media_type = MediaType::from_mime(Mime::from_str(media_type)?);

    let reader = response.into_reader();
    let reader = StreamReader::piped_thread(Box::new(reader))?;
    Ok(Input {
        name: http_url_str.to_owned(),
        media_type,
        reader,
        initial_size,
    })
}

fn open_data_url_str(data_url_str: &str) -> anyhow::Result<Input> {
    // TODO: `DataUrl` should really implement `std::error::Error`.
    let data_url =
        DataUrl::process(data_url_str).map_err(|e| anyhow!("invalid data URL syntax: {:?}", e))?;
    // TODO: `DataUrl` should really really implement `std::error::Error`.
    let (body, fragment) = data_url
        .decode_to_vec()
        .map_err(|_| anyhow!("invalid base64 encoding"))?;

    if fragment.is_some() {
        return Err(anyhow!("data urls with fragments are unsupported"));
    }

    // Awkwardly convert from `data_url::Mime` to `mime::Mime`.
    // TODO: Consider submitting patches to `data_url` to streamline this.
    let media_type =
        MediaType::from_mime(Mime::from_str(&data_url.mime_type().to_string()).unwrap());

    let reader = StreamReader::bytes(&body)?;
    Ok(Input {
        name: data_url_str.to_owned(),
        reader,
        media_type,
        initial_size: Some(data_url_str.len().try_into().unwrap()),
    })
}

// Handle URLs of the form `scp://[user@]host[:port][/path]`.
#[cfg(feature = "ssh2")]
fn open_scp_url(scp_url: &Url) -> anyhow::Result<Input> {
    if scp_url.query().is_some() || scp_url.fragment().is_some() {
        return Err(anyhow!("scp URL should only contain a socket address, optional username, optional password, and optional path"));
    }

    let host_str = match scp_url.host_str() {
        Some(host_str) => host_str,
        None => return Err(anyhow!("ssh URL should have a host")),
    };
    let port = match scp_url.port() {
        Some(port) => port,
        None => 22, // default ssh port
    };
    let tcp = TcpStream::connect((host_str, port))?;
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();

    let username = if scp_url.username().is_empty() {
        whoami::username()
    } else {
        scp_url.username().to_owned()
    };
    let username = percent_decode(username.as_bytes()).decode_utf8()?;

    if let Some(password) = scp_url.password() {
        let password = percent_decode(password.as_bytes()).decode_utf8()?;
        sess.userauth_password(&username, &password)?;
    } else {
        sess.userauth_agent(&username)?;
    }

    assert!(sess.authenticated());

    let path = Path::new(scp_url.path());
    let (channel, stat) = sess.scp_recv(path)?;
    let reader = StreamReader::piped_thread(Box::new(channel))?;
    let media_type = MediaType::from_extension(path.extension());
    Ok(Input {
        name: scp_url.as_str().to_owned(),
        reader,
        media_type,
        initial_size: Some(stat.size()),
    })
}

fn open_path(path: &Path) -> anyhow::Result<Input> {
    let name = path_to_name("file", path)?;
    // TODO: Should we have our own error type?
    let file = File::open(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
    if path.extension() == Some(Path::new("gz").as_os_str()) {
        // TODO: We shouldn't really need to allocate a `PathBuf` here.
        let path = path.with_extension("");
        let media_type = MediaType::from_extension(path.extension());
        let initial_size = None;
        let reader = GzDecoder::new(file);
        let reader = StreamReader::piped_thread(Box::new(reader))?;
        Ok(Input {
            name,
            reader,
            media_type,
            initial_size,
        })
    } else {
        let media_type = MediaType::from_extension(path.extension());
        let initial_size = Some(file.metadata()?.len());
        let reader = StreamReader::file(file);
        Ok(Input {
            name,
            reader,
            media_type,
            initial_size,
        })
    }
}

#[cfg(not(windows))]
fn spawn_child(os: &OsStr, lossy: &str) -> anyhow::Result<Input> {
    use std::process::{Command, Stdio};
    assert!(lossy.starts_with("$("));
    if !lossy.ends_with(')') {
        return Err(anyhow!("child string must end in ')'"));
    }
    let s = if let Some(s) = os.to_str() {
        s
    } else {
        return Err(anyhow!("Non-UTF-8 child strings not yet supported"));
    };
    let words = shell_words::split(&s[2..s.len() - 1])?;
    let (first, rest) = words
        .split_first()
        .ok_or_else(|| anyhow!("child stream specified with '(...)' must contain a command"))?;
    let child = Command::new(first)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let reader = StreamReader::child_stdout(child.stdout.unwrap());
    Ok(Input {
        name: s.to_owned(),
        reader,
        media_type: MediaType::unknown(),
        initial_size: None,
    })
}
