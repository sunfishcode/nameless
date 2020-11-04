// absolute path - character device
// pair of paths? - fifo pair?
// listen unix-domain socket
// accept unix-domain socket

use anyhow::anyhow;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::path::{Component, Path};

pub(crate) fn path_to_name(scheme: &str, path: &Path) -> anyhow::Result<String> {
    // FIXME: Windows. Drive letters and potentially-ill-formed UTF-16.
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
        let result = percent_encode(path.as_os_str().as_bytes(), NON_ALPHANUMERIC).to_string();
        let display = path.display().to_string();
        if result == display {
            Ok(result)
        } else {
            Err(anyhow!(
                "not supported yet: non-alphanumeric relative paths: {}",
                display
            ))
        }
    }
}

#[test]
fn test_path_to_name() {
    assert_eq!(path_to_name("file", Path::new("/")).unwrap(), "/");
    assert_eq!(path_to_name("file", Path::new("/foo")).unwrap(), "/foo");
    assert_eq!(
        path_to_name("file", Path::new("/foo:bar")).unwrap(),
        "file:///foo%3Abar"
    );
    assert_eq!(path_to_name("file", Path::new("foo")).unwrap(), "foo");
    // FIXME: Redo how relative paths are handled.
    //assert_eq!(path_to_name("file", Path::new("./foo")).unwrap(), "./foo");
    //assert_eq!(
    //    path_to_name("file", OsStr::from_bytes(b"f\xffoo").as_ref()).unwrap(),
    //    "\"./f\\u{fffd}oo\""
    //);
}