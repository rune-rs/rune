//! Error types used by rune alloc.

use core::fmt;

use crate::alloc::AllocError;

/// An error type returned when a custom error is available alongside an allocation error.
#[derive(Debug)]
pub enum CustomError<E> {
    /// Custom error being returned.
    Custom(E),
    /// Try reserve error being returned.
    Error(Error),
}

impl<E> From<Error> for CustomError<E> {
    fn from(error: Error) -> Self {
        CustomError::Error(error)
    }
}

impl<E> From<AllocError> for CustomError<E> {
    fn from(error: AllocError) -> Self {
        CustomError::Error(Error::from(error))
    }
}

/// The error type for methods which allocate or reserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    #[doc(hidden)]
    CapacityOverflow,

    /// Error when computing layout.
    #[doc(hidden)]
    LayoutError,

    /// Error during internal formatting.
    #[doc(hidden)]
    FormatError,

    /// The memory allocator returned an error
    #[doc(hidden)]
    AllocError {
        /// The layout of the allocation request that failed.
        error: AllocError,
    },
}

impl From<AllocError> for Error {
    #[inline]
    fn from(error: AllocError) -> Self {
        Error::AllocError { error }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CapacityOverflow => write!(f, "Capacity overflow"),
            Error::LayoutError => write!(f, "Layout error"),
            Error::FormatError => write!(f, "Format error"),
            Error::AllocError { error } => error.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for Error {
    fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
        match self {
            Error::AllocError { error } => Some(error),
            _ => None,
        }
    }
}
