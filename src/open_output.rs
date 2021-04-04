use crate::{path_to_name::path_to_name, Type};
use anyhow::anyhow;
use flate2::{write::GzEncoder, Compression};
use io_handles::WriteHandle;
use std::{fs::File, path::Path};
use url::Url;

pub(crate) struct Output {
    pub(crate) name: String,
    pub(crate) writer: WriteHandle,
    pub(crate) type_: Type,
}

pub(crate) fn open_output(s: &str, type_: Type) -> anyhow::Result<Output> {
    // If we can parse it as a URL, treat it as such.
    if let Ok(url) = Url::parse(s) {
        return open_url(url, type_);
    }

    // Special-case "-" to mean stdout.
    if s == "-" {
        return acquire_stdout(type_);
    }

    // Strings beginning with "$(" are commands.
    #[cfg(not(windows))]
    if s.starts_with("$(") {
        return spawn_child(s, type_);
    }

    // Otherwise try opening it as a path in the filesystem namespace.
    open_path(Path::new(s), type_)
}

fn acquire_stdout(type_: Type) -> anyhow::Result<Output> {
    let stdout = WriteHandle::stdout()?;

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
            WriteHandle::piped_thread(Box::new(GzEncoder::new(file, Compression::new(6))))?;
        Ok(Output {
            name,
            writer,
            type_,
        })
    } else {
        let type_ = Type::merge(type_, Type::from_extension(path.extension()));
        let writer = WriteHandle::file(file);
        Ok(Output {
            name,
            writer,
            type_,
        })
    }
}

#[cfg(not(windows))]
fn spawn_child(s: &str, type_: Type) -> anyhow::Result<Output> {
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
    let writer = WriteHandle::child_stdin(child.stdin.unwrap());
    Ok(Output {
        name: s.to_owned(),
        writer,
        type_,
    })
}
