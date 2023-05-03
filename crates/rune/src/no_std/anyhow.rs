use core::fmt;

use crate::no_std::error::Error as StdError;

/// Type-erased error produced in no-std environments.
#[derive(Debug)]
pub struct Error {}

impl Error {
    pub(crate) fn msg<D>(_: D) -> Self
    where
        D: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {}
    }

    pub(crate) fn downcast<E>(self) -> Result<E, Self>
    where
        E: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Err(self)
    }

    pub(crate) fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An error occured")
    }
}

impl<E> From<E> for Error
where
    E: StdError + Send + Sync + 'static,
{
    #[cold]
    fn from(_: E) -> Self {
        Error {}
    }
}
