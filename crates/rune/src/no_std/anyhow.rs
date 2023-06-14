use core::fmt;

use crate::no_std::error::Error as StdError;

/// Type-erased error produced in no-std environments.
#[derive(Debug)]
pub struct Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An error occurred")
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
