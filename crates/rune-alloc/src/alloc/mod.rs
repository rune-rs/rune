//! Allocated types.

pub use self::allocator::Allocator;
mod allocator;

pub use self::global::Global;
mod global;

use core::alloc::Layout;
use core::convert::Infallible;
use core::fmt;

use crate::error::{CustomError, Error};

/// Error raised while allocating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocError {
    pub(crate) layout: Layout,
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to allocate {} bytes of memory",
            self.layout.size()
        )
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for AllocError {}

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
