//! Built-in formatting utilities.

mod impls;

use core::fmt::{self, Arguments};

use crate::borrow::TryToOwned;
use crate::error::Error;
use crate::string::String;

/// Fallible write formatting implementation.
pub trait TryWrite {
    /// Writes a string slice into this writer, returning whether the write
    /// succeeded.
    ///
    /// This method can only succeed if the entire string slice was successfully
    /// written, and this method will not return until all data has been
    /// written or an error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an instance of [`Error`] on error.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::fmt::TryWrite;
    /// use rune::alloc::{String, Error};
    ///
    /// fn writer<W: TryWrite>(f: &mut W, s: &str) -> Result<(), Error> {
    ///     f.try_write_str(s)
    /// }
    ///
    /// let mut buf = String::new();
    /// writer(&mut buf, "hola")?;
    /// assert_eq!(&buf, "hola");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_write_str(&mut self, s: &str) -> Result<(), Error>;

    /// Writes a [`char`] into this writer, returning whether the write succeeded.
    ///
    /// A single [`char`] may be encoded as more than one byte.
    /// This method can only succeed if the entire byte sequence was successfully
    /// written, and this method will not return until all data has been
    /// written or an error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an instance of [`Error`] on error.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::fmt::TryWrite;
    /// use rune::alloc::{String, Error};
    ///
    /// fn writer<W: TryWrite>(f: &mut W, c: char) -> Result<(), Error> {
    ///     f.try_write_char(c)
    /// }
    ///
    /// let mut buf = String::new();
    /// writer(&mut buf, 'a')?;
    /// writer(&mut buf, 'b')?;
    /// assert_eq!(&buf, "ab");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_write_char(&mut self, c: char) -> Result<(), Error> {
        self.try_write_str(c.encode_utf8(&mut [0; 4]))
    }

    #[inline]
    #[doc(hidden)]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), Error>
    where
        Self: Sized,
    {
        struct Writer<'a> {
            target: &'a mut dyn TryWrite,
            error: Option<Error>,
        }

        impl fmt::Write for Writer<'_> {
            #[inline]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                if let Err(error) = (*self.target).try_write_str(s) {
                    self.error = Some(error);
                }

                Ok(())
            }

            #[inline]
            fn write_char(&mut self, c: char) -> fmt::Result {
                if let Err(error) = (*self.target).try_write_char(c) {
                    self.error = Some(error);
                }

                Ok(())
            }
        }

        let mut writer = Writer {
            target: self,
            error: None,
        };

        if let Err(fmt::Error) = fmt::write(&mut writer, args) {
            return Err(Error::FormatError);
        }

        if let Some(error) = writer.error {
            Err(error)
        } else {
            Ok(())
        }
    }
}

/// The `format` function takes an [`Arguments`] struct and returns the
/// resulting formatted string.
///
/// The [`Arguments`] instance can be created with the [`format_args!`] macro.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rune::alloc::fmt;
///
/// let s = fmt::try_format(format_args!("Hello, {}!", "world"))?;
/// assert_eq!(s, "Hello, world!");
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// Please note that using [`try_format!`] might be preferable. Example:
///
/// ```
/// use rune::alloc::try_format;
///
/// let s = try_format!("Hello, {}!", "world");
/// assert_eq!(s, "Hello, world!");
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`format_args!`]: core::format_args
/// [`try_format!`]: try_format!
#[inline]
pub fn try_format(args: Arguments<'_>) -> Result<String, Error> {
    #[cfg(rune_nightly)]
    fn estimated_capacity(args: &Arguments<'_>) -> usize {
        args.estimated_capacity()
    }

    #[cfg(not(rune_nightly))]
    fn estimated_capacity(_: &Arguments<'_>) -> usize {
        0
    }

    fn format_inner(args: Arguments<'_>) -> Result<String, Error> {
        let capacity = estimated_capacity(&args);
        let mut output = String::try_with_capacity(capacity)?;
        output.write_fmt(args)?;
        Ok(output)
    }

    match args.as_str() {
        Some(string) => string.try_to_owned(),
        None => format_inner(args),
    }
}
