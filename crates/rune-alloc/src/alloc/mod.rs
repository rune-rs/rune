//! Allocated types.

pub(crate) mod raw_vec;

pub use self::borrow::TryToOwned;
pub(crate) mod borrow;

pub use self::allocator::{AllocError, Allocator, Global};
pub(crate) mod allocator;

pub use self::boxed::Box;
pub mod boxed;

pub(crate) mod btree;

pub use self::hashbrown::HashMap;
pub mod hashbrown;

pub use self::vec::Vec;
pub mod vec;

pub use self::vec_deque::VecDeque;
pub mod vec_deque;

pub use self::try_clone::{TryClone, TryCopy};
mod try_clone;

pub use self::try_extend::TryExtend;
mod try_extend;

pub use self::try_from_iterator::{TryFromIterator, TryFromIteratorIn};
mod try_from_iterator;

pub use self::string::String;
pub mod string;

mod slice;
pub mod str;

#[cfg(test)]
pub(crate) mod testing;

use core::convert::Infallible;
use core::fmt;

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
            Error::AllocError { error } => error.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl ::rust_std::error::Error for Error {
    fn source(&self) -> Option<&(dyn ::rust_std::error::Error + 'static)> {
        match self {
            Error::AllocError { error } => Some(error),
            _ => None,
        }
    }
}

pub(crate) trait SizedTypeProperties: Sized {
    const IS_ZST: bool = core::mem::size_of::<Self>() == 0;
    const NEEDS_DROP: bool = core::mem::needs_drop::<Self>();
}

impl<T> SizedTypeProperties for T {}

#[inline(always)]
pub(crate) fn into_ok<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => match error {},
    }
}

#[inline(always)]
pub(crate) fn into_ok_try<T>(result: Result<T, CustomError<Infallible>>) -> Result<T, Error> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => match error {
            CustomError::Error(error) => Err(error),
            CustomError::Custom(error) => match error {},
        },
    }
}
