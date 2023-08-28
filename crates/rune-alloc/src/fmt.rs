//! Built-in formatting utilities.

use core::fmt;

use crate::alloc::Error;

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
    /// use std::fmt::{Error, Write};
    ///
    /// fn writer<W: Write>(f: &mut W, s: &str) -> Result<(), Error> {
    ///     f.write_str(s)
    /// }
    ///
    /// let mut buf = String::new();
    /// writer(&mut buf, "hola").unwrap();
    /// assert_eq!(&buf, "hola");
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
    /// use std::fmt::{Error, Write};
    ///
    /// fn writer<W: Write>(f: &mut W, c: char) -> Result<(), Error> {
    ///     f.write_char(c)
    /// }
    ///
    /// let mut buf = String::new();
    /// writer(&mut buf, 'a').unwrap();
    /// writer(&mut buf, 'b').unwrap();
    /// assert_eq!(&buf, "ab");
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

        fmt::write(&mut writer, args).unwrap();

        if let Some(error) = writer.error {
            Err(error)
        } else {
            Ok(())
        }
    }
}
