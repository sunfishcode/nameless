use percent_encoding::{percent_encode, NON_ALPHANUMERIC, CONTROLS};
use std::path::{Path, Component};

pub(crate) fn path_url(path: &Path) -> String {
    // FIXME: Windows
    use std::os::unix::ffi::OsStrExt;
    if path.is_absolute() {
        let mut result = "file://".to_string();
        let mut components = path.components();
        assert!(components.next().unwrap() == Component::RootDir);
        if let Some(component) = components.next() {
            result += "/";
            result += &percent_encode(component.as_os_str().as_bytes(), NON_ALPHANUMERIC).to_string();
            for component in components {
                result += "/";
                result += &percent_encode(component.as_os_str().as_bytes(), NON_ALPHANUMERIC).to_string();
            }
        } else {
            result += "/";
        }
        result
    } else {
        percent_encode(&path.as_os_str().as_bytes(), CONTROLS).to_string()
    }
}

#[test]
fn path_urls() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    assert_eq!(path_url(Path::new("/")), "file:///");
    assert_eq!(path_url(Path::new("/foo")), "file:///foo");
    assert_eq!(path_url(Path::new("foo")), "foo");
    assert_eq!(path_url(Path::new("./foo")), "./foo");
    assert_eq!(path_url(OsStr::from_bytes(b"f\xffoo").as_ref()), "./foo");
}
