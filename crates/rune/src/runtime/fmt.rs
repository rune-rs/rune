use core::fmt;

use crate as rune;

use crate::no_std::string::String;
use crate::Any;

/// A formatter for the rune virtual machine.
///
/// This is used as a receiver to functions implementing the [`STRING_DEBUG`]
/// and [`STRING_DISPLAY`] protocols.
///
/// [`STRING_DEBUG`]: crate::runtime::Protocol::STRING_DEBUG
/// [`STRING_DISPLAY`]: crate::runtime::Protocol::STRING_DISPLAY
#[derive(Any, Default)]
#[rune(item = ::std::fmt)]
pub struct Formatter {
    pub(crate) string: String,
    pub(crate) buf: String,
}

impl Formatter {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            string: String::new(),
            buf: String::new(),
        }
    }

    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            string: String::with_capacity(capacity),
            buf: String::new(),
        }
    }

    #[inline]
    pub(crate) fn parts_mut(&mut self) -> (&mut String, &str) {
        (&mut self.string, &self.buf)
    }

    #[inline]
    pub(crate) fn buf_mut(&mut self) -> &mut String {
        &mut self.buf
    }

    #[inline]
    pub(crate) fn push(&mut self, c: char) {
        self.string.push(c);
    }

    #[inline]
    pub(crate) fn push_str(&mut self, s: &str) {
        self.string.push_str(s);
    }

    #[inline]
    pub(crate) fn into_string(self) -> String {
        self.string
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        &self.string
    }
}

impl fmt::Write for Formatter {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.string.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.string.push(c);
        Ok(())
    }
}
