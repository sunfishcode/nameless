use crate::MediaType;
use clap::TryFromOsArg;
use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt,
    marker::PhantomData,
};

#[doc(hidden)]
#[derive(Debug)]
pub struct Never {}

impl Error for Never {}

impl fmt::Display for Never {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!()
    }
}

#[doc(hidden)]
pub trait FromLazyOutput {
    type Err;

    fn from_lazy_output(name: OsString, media_type: MediaType) -> Result<Self, Self::Err>
    where
        Self: Sized;
}

/// A placeholder for an output stream which is created lazily. It is created
/// when `materialize` is called.
pub struct LazyOutput<T: FromLazyOutput> {
    name: OsString,
    _phantom: PhantomData<T>,
}

impl<T: FromLazyOutput> LazyOutput<T> {
    /// Consume `self` and materialize an output stream.
    #[inline]
    pub fn materialize(self, media_type: MediaType) -> Result<T, T::Err> {
        T::from_lazy_output(self.name, media_type)
    }
}

impl<T: FromLazyOutput> TryFromOsArg for LazyOutput<T> {
    type Error = Never;

    #[inline]
    fn try_from_os_str_arg(os: &OsStr) -> Result<Self, Never> {
        Ok(Self {
            name: os.to_owned(),
            _phantom: PhantomData::default(),
        })
    }
}
