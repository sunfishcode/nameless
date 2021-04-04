use crate::Type;
use std::{error::Error, fmt, marker::PhantomData, str::FromStr};

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

    fn from_lazy_output(name: String, type_: Type) -> Result<Self, Self::Err>
    where
        Self: Sized;
}

/// A placeholder for an output stream which is created lazily. It is created
/// when `materialize` is called.
pub struct LazyOutput<T: FromLazyOutput> {
    name: String,
    _phantom: PhantomData<T>,
}

impl<T: FromLazyOutput> LazyOutput<T> {
    /// Consume `self` and materialize an output stream.
    #[inline]
    pub fn materialize(self, type_: Type) -> Result<T, T::Err> {
        T::from_lazy_output(self.name, type_)
    }
}

impl<T: FromLazyOutput> FromStr for LazyOutput<T> {
    type Err = Never;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Never> {
        Ok(Self {
            name: s.to_owned(),
            _phantom: PhantomData::default(),
        })
    }
}
