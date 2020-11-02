/// This struct encapsulates the name of an entity whose name is being
/// hidden in the `nameless` API. It can be written to an `OutputByteStream`
/// but it otherwise entirely opaque.
pub struct Pseudonym {
    pub(crate) name: String,
}

impl Pseudonym {
    pub(crate) fn new(name: String) -> Self {
        Self { name }
    }
}
