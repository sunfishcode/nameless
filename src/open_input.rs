use crate::{path_to_name::path_to_name, Mime, Type};
use anyhow::anyhow;
use data_url::DataUrl;
use flate2::read::GzDecoder;
use io_handles::ReadHandle;
use std::{fs::File, path::Path, str::FromStr};
use url::Url;

pub(crate) struct Input {
    pub(crate) name: String,
    pub(crate) reader: ReadHandle,
    pub(crate) type_: Type,
    pub(crate) initial_size: Option<u64>,
}

pub(crate) fn open_input(s: &str) -> anyhow::Result<Input> {
    // If we can parse it as a URL, treat it as such.
    if let Ok(url) = Url::parse(s) {
        return open_url(url);
    }

    // Special-case "-" to mean stdin.
    if s == "-" {
        return acquire_stdin();
    }

    // Strings beginning with "$(" are commands.
    #[cfg(not(windows))]
    if s.starts_with("$(") {
        return spawn_child(s);
    }

    // Otherwise try opening it as a path in the filesystem namespace.
    open_path(Path::new(s))
}

fn acquire_stdin() -> anyhow::Result<Input> {
    let reader = ReadHandle::stdin()?;
    Ok(Input {
        name: "-".to_owned(),
        reader,
        type_: Type::unknown(),
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
        other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
    }
}

fn open_http_url_str(http_url_str: &str) -> anyhow::Result<Input> {
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

    let reader = response.into_reader();
    let reader = ReadHandle::piped_thread(Box::new(reader))?;
    Ok(Input {
        name: http_url_str.to_owned(),
        type_,
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
    let type_ = Type::from_mime(Mime::from_str(&data_url.mime_type().to_string()).unwrap());

    let reader = ReadHandle::bytes(&body)?;
    Ok(Input {
        name: data_url_str.to_owned(),
        reader,
        type_,
        initial_size: Some(data_url_str.len() as u64),
    })
}

fn open_path(path: &Path) -> anyhow::Result<Input> {
    let name = path_to_name("file", path)?;
    // TODO: Should we have our own error type?
    let file = File::open(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
    if path.extension() == Some(Path::new("gz").as_os_str()) {
        // TODO: We shouldn't really need to allocate a `PathBuf` here.
        let path = path.with_extension("");
        let type_ = Type::from_extension(path.extension());
        let initial_size = None;
        let reader = GzDecoder::new(file);
        let reader = ReadHandle::piped_thread(Box::new(reader))?;
        Ok(Input {
            name,
            reader,
            type_,
            initial_size,
        })
    } else {
        let type_ = Type::from_extension(path.extension());
        let initial_size = Some(file.metadata()?.len());
        let reader = ReadHandle::file(file);
        Ok(Input {
            name,
            reader,
            type_,
            initial_size,
        })
    }
}

#[cfg(not(windows))]
fn spawn_child(s: &str) -> anyhow::Result<Input> {
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
    let reader = ReadHandle::child_stdout(child.stdout.unwrap());
    Ok(Input {
        name: s.to_owned(),
        reader,
        type_: Type::unknown(),
        initial_size: None,
    })
}
