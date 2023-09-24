//! Our own private error trait for use in no-std environments.

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

pub trait Error {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[cfg(feature = "alloc")]
impl<E> Error for Box<E>
where
    E: Error,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}

impl Error for ::rune_alloc::Error {}
