use crate::{path_to_name::path_to_name, Type};
use anyhow::anyhow;
use flate2::{write::GzEncoder, Compression};
use io_streams::StreamWriter;
use std::{ffi::OsStr, fs::File, path::Path};
use url::Url;

pub(crate) struct Output {
    pub(crate) name: String,
    pub(crate) writer: StreamWriter,
    pub(crate) type_: Type,
}

pub(crate) fn open_output(os: &OsStr, type_: Type) -> anyhow::Result<Output> {
    if let Some(s) = os.to_str() {
        // If we can parse it as a URL, treat it as such.
        if let Ok(url) = Url::parse(s) {
            return open_url(url, type_);
        }

        // Special-case "-" to mean stdout.
        if s == "-" {
            return acquire_stdout(type_);
        }
    }

    {
        let lossy = os.to_string_lossy();

        // Strings beginning with "$(" are commands.
        #[cfg(not(windows))]
        if lossy.starts_with("$(") {
            return spawn_child(os, &lossy, type_);
        }
    }

    // Otherwise try opening it as a path in the filesystem namespace.
    open_path(Path::new(os), type_)
}

fn acquire_stdout(type_: Type) -> anyhow::Result<Output> {
    let stdout = StreamWriter::stdout()?;

    Ok(Output {
        name: "-".to_string(),
        writer: stdout,
        type_,
    })
}

fn open_url(url: Url, type_: Type) -> anyhow::Result<Output> {
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
            open_path(
                &url.to_file_path()
                    .map_err(|_: ()| anyhow!("unknown file URL weirdness"))?,
                type_,
            )
        }
        "data" => Err(anyhow!("output to data URL isn't possible")),
        other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
    }
}

fn open_path(path: &Path, type_: Type) -> anyhow::Result<Output> {
    let name = path_to_name("file", path)?;
    let file = File::create(path).map_err(|err| anyhow!("{}: {}", path.display(), err))?;
    if path.extension() == Some(Path::new("gz").as_os_str()) {
        // TODO: We shouldn't really need to allocate a `PathBuf` here.
        let path = path.with_extension("");
        let type_ = Type::merge(type_, Type::from_extension(path.extension()));
        // 6 is the default gzip compression level.
        let writer =
            StreamWriter::piped_thread(Box::new(GzEncoder::new(file, Compression::new(6))))?;
        Ok(Output {
            name,
            writer,
            type_,
        })
    } else {
        let type_ = Type::merge(type_, Type::from_extension(path.extension()));
        let writer = StreamWriter::file(file);
        Ok(Output {
            name,
            writer,
            type_,
        })
    }
}

#[cfg(not(windows))]
fn spawn_child(os: &OsStr, lossy: &str, type_: Type) -> anyhow::Result<Output> {
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
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;
    let writer = StreamWriter::child_stdin(child.stdin.unwrap());
    Ok(Output {
        name: lossy.to_owned(),
        writer,
        type_,
    })
}
