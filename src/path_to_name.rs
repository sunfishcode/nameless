use anyhow::anyhow;
use std::{path::Path, str};
#[cfg(not(windows))]
use {
    percent_encoding::{percent_encode, NON_ALPHANUMERIC},
    std::path::Component,
};

#[cfg(not(windows))]
pub(crate) fn path_to_name(scheme: &str, path: &Path) -> anyhow::Result<String> {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    if path.is_absolute() {
        let mut result = String::new();
        let mut components = path.components();
        assert!(components.next().unwrap() == Component::RootDir);
        if let Some(component) = components.next() {
            result += "/";
            result +=
                &percent_encode(component.as_os_str().as_bytes(), NON_ALPHANUMERIC).to_string();
            for component in components {
                result += "/";
                result +=
                    &percent_encode(component.as_os_str().as_bytes(), NON_ALPHANUMERIC).to_string();
            }
        } else {
            result += "/";
        }
        if result == path.display().to_string() {
            Ok(result)
        } else {
            Ok(format!("{}://{}", scheme, result))
        }
    } else {
        let result = str::from_utf8(path.as_os_str().as_bytes())
            .map_err(|_| anyhow!("not supported yet: non-UTF-8 relative paths",))?
            .escape_default()
            .to_string();
        if result.contains(':') {
            return Err(anyhow!("not supported yet: strings contains `:`"));
        }
        let display = path.display().to_string();
        if result == display {
            Ok(result)
        } else {
            Err(anyhow!(
                "not supported yet: \"interesting\" strings: {}",
                result
            ))
        }
    }
}

#[cfg(windows)]
pub(crate) fn path_to_name(_scheme: &str, path: &Path) -> anyhow::Result<String> {
    if path.is_absolute() {
        Ok(url::Url::from_file_path(path)
            .map_err(|_| {
                anyhow!(
                    "not supported yet: \"interesting\" strings: {}",
                    path.display()
                )
            })?
            .into_string())
    } else {
        Err(anyhow!("not supported yet: non-UTF-8 relative paths",))
    }
}

#[test]
#[cfg_attr(windows, ignore)] // TODO: Improve path handling on Windows.
fn test_path_to_name() {
    assert_eq!(path_to_name("file", Path::new("/")).unwrap(), "/");
    assert_eq!(path_to_name("file", Path::new("/foo")).unwrap(), "/foo");
    assert_eq!(
        path_to_name("file", Path::new("/foo:bar")).unwrap(),
        "file:///foo%3Abar"
    );
    assert_eq!(path_to_name("file", Path::new("foo")).unwrap(), "foo");
    // TODO: Redo how relative paths are handled.
    // assert_eq!(path_to_name("file", Path::new("./foo")).unwrap(), "./foo");
    // assert_eq!(
    //    path_to_name("file", OsStr::from_bytes(b"f\xffoo").as_ref()).unwrap(),
    //    "\"./f\\u{fffd}oo\""
    //);
}
