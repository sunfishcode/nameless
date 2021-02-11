use crate::path_to_name::path_to_name;
use anyhow::anyhow;
use io_streams::StreamDuplexer;
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::{
    net::{TcpListener, TcpStream},
    path::Path,
};
use url::Url;

pub(crate) struct Interactive {
    pub(crate) name: String,
    pub(crate) duplexer: StreamDuplexer,
}

pub(crate) fn open_interactive(s: &str) -> anyhow::Result<Interactive> {
    // If we can parse it as a URL, treat it as such.
    if let Ok(url) = Url::parse(s) {
        return open_url(url);
    }

    // Special-case "-" to mean (stdin, stdout).
    if s == "-" {
        return acquire_stdin_stdout();
    }

    // Strings beginning with "$(" are commands.
    #[cfg(not(windows))]
    if s.starts_with("$(") {
        return spawn_child(s);
    }

    // Otherwise try opening it as a path in the filesystem namespace.
    open_path(Path::new(s))
}

fn acquire_stdin_stdout() -> anyhow::Result<Interactive> {
    let duplexer = StreamDuplexer::stdin_stdout()?;
    Ok(Interactive {
        name: "-".to_owned(),
        duplexer,
    })
}

fn open_url(url: Url) -> anyhow::Result<Interactive> {
    match url.scheme() {
        "connect" => open_connect_url_str(url),
        "accept" => open_accept_url_str(url),
        scheme @ "http" | scheme @ "https" | scheme @ "file" | scheme @ "data" => {
            Err(anyhow!("non-interactive URL scheme \"{}\"", scheme))
        }
        other => Err(anyhow!("unsupported URL scheme \"{}\"", other)),
    }
}

fn open_connect_url_str(url: Url) -> anyhow::Result<Interactive> {
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(anyhow!("connect URL should only contain a socket address"));
    }

    if url.path().is_empty() {
        let port = match url.port() {
            Some(port) => port,
            None => return Err(anyhow!("TCP connect URL should have a port")),
        };
        let host_str = match url.host_str() {
            Some(host_str) => host_str,
            None => return Err(anyhow!("TCP connect URL should have a host")),
        };

        let duplexer = TcpStream::connect(format!("{}:{}", host_str, port))?;
        let duplexer = StreamDuplexer::tcp_stream(duplexer);

        return Ok(Interactive {
            name: url.to_string(),
            duplexer,
        });
    }

    #[cfg(unix)]
    {
        if url.port().is_some() || url.host_str().is_some() {
            return Err(anyhow!(
                "Unix-domain connect URL should only contain a path"
            ));
        }

        let duplexer = UnixStream::connect(url.path())?;
        let duplexer = StreamDuplexer::unix_stream(duplexer);

        return Ok(Interactive {
            name: url.to_string(),
            duplexer,
        });
    }

    #[cfg(windows)]
    return Err(anyhow!("Unsupported connect URL: {}", url));
}

fn open_accept_url_str(url: Url) -> anyhow::Result<Interactive> {
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(anyhow!("accept URL should only contain a socket address"));
    }

    if url.path().is_empty() {
        let port = match url.port() {
            Some(port) => port,
            None => return Err(anyhow!("accept URL should have a port")),
        };
        let host_str = match url.host_str() {
            Some(host_str) => host_str,
            None => return Err(anyhow!("accept URL should have a host")),
        };

        let listener = TcpListener::bind(format!("{}:{}", host_str, port))?;

        let (duplexer, addr) = listener.accept()?;
        let duplexer = StreamDuplexer::tcp_stream(duplexer);

        return Ok(Interactive {
            name: format!("accept://{}", addr),
            duplexer,
        });
    }

    #[cfg(unix)]
    {
        if url.port().is_some() || url.host_str().is_some() {
            return Err(anyhow!(
                "Unix-domain connect URL should only contain a path"
            ));
        }

        let listener = UnixListener::bind(url.path())?;

        let (duplexer, addr) = listener.accept()?;
        let duplexer = StreamDuplexer::unix_stream(duplexer);
        let name = path_to_name("accept", addr.as_pathname().unwrap())?;

        return Ok(Interactive { name, duplexer });
    }

    #[cfg(windows)]
    return Err(anyhow!("Unsupported connect URL: {}", url));
}

fn open_path(_path: &Path) -> anyhow::Result<Interactive> {
    Err(anyhow!(
        "interactive filesystem paths not supported on Windows yet"
    ))
}

#[cfg(not(windows))]
fn spawn_child(s: &str) -> anyhow::Result<Interactive> {
    use std::process::Command;
    assert!(s.starts_with("$("));
    if !s.ends_with(')') {
        return Err(anyhow!("child string must end in ')'"));
    }
    let words = shell_words::split(&s[2..s.len() - 1])?;
    let (first, rest) = words
        .split_first()
        .ok_or_else(|| anyhow!("child stream specified with '(...)' must contain a command"))?;
    let mut command = Command::new(first);
    command.args(rest);
    let duplexer = StreamDuplexer::duplex_with_command(command)?;
    Ok(Interactive {
        name: s.to_owned(),
        duplexer,
    })
}
