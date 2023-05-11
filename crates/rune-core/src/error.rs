//! Our own private error trait for use in no-std environments.

use alloc::boxed::Box;

pub trait Error {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl<E> Error for Box<E>
where
    E: Error,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}
