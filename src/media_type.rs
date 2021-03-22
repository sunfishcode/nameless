use mime::Mime;
use std::ffi::OsStr;
use std::str::FromStr;

/// The type of content in a stream. This can be either a Media Type
/// (aka Mime Type) or a filename extension, both, or neither if nothing
/// is known.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaType {
    mime: Mime,
    extension: String,
}

impl MediaType {
    /// Construct a type representing completely unknown contents.
    pub fn unknown() -> Self {
        Self {
            mime: mime::STAR_STAR,
            extension: String::new(),
        }
    }

    /// Construct a type representing plain text (UTF-8) contents.
    pub fn text() -> Self {
        Self {
            mime: mime::TEXT_PLAIN_UTF_8,
            extension: String::new(),
        }
    }

    /// Construct a type representing the given Media Type.
    pub fn from_mime(mime: Mime) -> Self {
        let extension = match mime_guess::get_mime_extensions(&mime) {
            Some(exts) => {
                if exts.len() == 1 {
                    exts[0].to_string()
                } else {
                    String::new()
                }
            }
            None => String::new(),
        };

        Self { mime, extension }
    }

    /// Construct a type representing the given filename extension.
    pub fn from_extension(extension: Option<&OsStr>) -> Self {
        if let Some(ext) = extension {
            if let Some(s) = ext.to_str() {
                let mut guesses = mime_guess::from_ext(s).iter();

                if let Some(first) = guesses.next() {
                    let mut merged = Self {
                        mime: first,
                        extension: s.to_string(),
                    };
                    for guess in guesses {
                        merged = merged.union(Self {
                            mime: guess,
                            extension: s.to_string(),
                        });
                    }
                    merged
                } else {
                    Self::unknown()
                }
            } else {
                Self::unknown()
            }
        } else {
            Self::unknown()
        }
    }

    /// Return the Media Type, which is "*/*" if unknown.
    #[inline]
    pub fn mime(&self) -> &Mime {
        &self.mime
    }

    /// Return the filename extension, which is empty if unknown.
    #[inline]
    pub fn extension(&self) -> &str {
        &self.extension
    }

    /// Return a type which is the generalization of `self` and `other`. Falls
    /// back to `MediaType::unknown()` if it cannot be determined.
    pub fn union(self, other: Self) -> Self {
        if self == other {
            self
        } else if other == MediaType::unknown() {
            self
        } else if self == MediaType::unknown() {
            other
        } else if self.mime.type_() == other.mime.type_()
            && self.mime.suffix() == other.mime.suffix()
            && self.mime.params().eq(other.mime.params())
        {
            if other.mime.subtype().as_str() == mime::STAR && other.mime.suffix().is_none() {
                self
            } else if self.mime.subtype().as_str() == mime::STAR && self.mime.suffix().is_none() {
                other
            } else {
                // Create a new mime value with the subtype replaced by star.
                let mut s = format!("{}/{}", self.mime.type_(), mime::STAR);
                if let Some(suffix) = self.mime.suffix() {
                    s += &format!("+{}", suffix);
                }
                if self.mime.params().next().is_some() {
                    for param in self.mime.params() {
                        s += &format!("; {}={}", param.0, param.1);
                    }
                }
                MediaType::from_mime(Mime::from_str(&s).unwrap())
            }
        } else if other == MediaType::text() {
            if self.mime.type_() == other.mime.type_() {
                self
            } else {
                MediaType::unknown()
            }
        } else if self == MediaType::text() {
            if self.mime.type_() == other.mime.type_() {
                other
            } else {
                MediaType::unknown()
            }
        } else {
            MediaType::unknown()
        }
    }
}

#[test]
fn mime_from_extension() {
    use std::path::Path;
    assert_eq!(MediaType::from_extension(None), MediaType::unknown());
    assert_eq!(
        MediaType::from_extension(Some(Path::new("jpg").as_ref())).mime(),
        &Mime::from_str("image/jpeg").unwrap()
    );
}

#[test]
fn mime_union() {
    assert_eq!(
        MediaType::from_mime(Mime::from_str("image/jpeg").unwrap())
            .union(MediaType::from_mime(Mime::from_str("image/png").unwrap()))
            .mime(),
        &Mime::from_str("image/*").unwrap()
    );
}
