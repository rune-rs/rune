//! String utilities.

use core::fmt;

#[cfg(test)]
use crate::alloc::testing::*;
use crate::{Error, String, TryWrite};

/// A trait for converting a value to a `String`.
///
/// This trait is automatically implemented for any type which implements the
/// [`Display`] trait. As such, `ToString` shouldn't be implemented directly:
/// [`Display`] should be implemented instead, and you get the `ToString`
/// implementation for free.
///
/// [`Display`]: core::fmt::Display
pub trait TryToString {
    #[cfg(test)]
    fn to_string(&self) -> String {
        self.try_to_string().abort()
    }

    /// Converts the given value to a `String`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune_alloc::{String, TryToString};
    ///
    /// let i = 5;
    /// let five = String::try_from("5")?;
    ///
    /// assert_eq!(five, i.try_to_string()?);
    /// # Ok::<_, rune_alloc::Error>(())
    /// ```
    fn try_to_string(&self) -> Result<String, Error>;
}

impl<T> TryToString for T
where
    T: fmt::Display,
{
    #[inline]
    fn try_to_string(&self) -> Result<String, Error> {
        let mut s = String::new();
        core::write!(s, "{}", self)?;
        Ok(s)
    }
}

impl TryToString for str {
    #[inline]
    fn try_to_string(&self) -> Result<String, Error> {
        String::try_from(self)
    }
}
