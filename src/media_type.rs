use mime::Mime;
use std::ffi::OsStr;

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
                let guess = mime_guess::from_ext(s);

                let mime = if guess.count() == 1 {
                    guess.first().unwrap()
                } else {
                    mime::STAR_STAR
                };

                Self {
                    mime,
                    extension: s.to_string(),
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
