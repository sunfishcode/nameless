use percent_encoding::{percent_encode, CONTROLS, NON_ALPHANUMERIC};
use std::path::{Component, Path};

pub(crate) fn path_url(path: &Path) -> String {
    // FIXME: Windows
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
            result
        } else {
            format!("file://{}", result)
        }
    } else {
        let result = percent_encode(&path.as_os_str().as_bytes(), CONTROLS).to_string();
        let display = path.display().to_string();
        if result == "-" {
            result
        } else if result == display {
            result
        } else {
            // FIXME: What should we do if the name has (a) invalid bytes or
            // (b) risky bytes like ` ` or `:`?
            format!("./{}", display)
        }
    }
}

#[test]
fn path_urls() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    assert_eq!(path_url(Path::new("/")), "/");
    assert_eq!(path_url(Path::new("/foo")), "/foo");
    assert_eq!(path_url(Path::new("/foo:bar")), "file:///foo%3Abar");
    assert_eq!(path_url(Path::new("foo")), "foo");
    assert_eq!(path_url(Path::new("./foo")), "./foo");
    assert_eq!(path_url(OsStr::from_bytes(b"f\xffoo").as_ref()), "./f�oo");
}
