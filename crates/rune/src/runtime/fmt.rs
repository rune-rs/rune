use crate as rune;

use crate::alloc::fmt::TryWrite;
use crate::alloc::{self, String};
use crate::Any;

/// A formatter for the rune virtual machine.
///
/// This is used as a receiver to functions implementing the [`STRING_DEBUG`]
/// and [`STRING_DISPLAY`] protocols.
///
/// [`STRING_DEBUG`]: crate::runtime::Protocol::STRING_DEBUG
/// [`STRING_DISPLAY`]: crate::runtime::Protocol::STRING_DISPLAY
#[derive(Any)]
#[rune(item = ::std::fmt)]
pub struct Formatter {
    pub(crate) string: String,
    pub(crate) buf: String,
}

impl Formatter {
    /// Construct a new empty formatter.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Formatter;
    ///
    /// let mut f = Formatter::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            string: String::new(),
            buf: String::new(),
        }
    }

    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> alloc::Result<Self> {
        Ok(Self {
            string: String::try_with_capacity(capacity)?,
            buf: String::new(),
        })
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
    pub(crate) fn push(&mut self, c: char) -> alloc::Result<()> {
        self.string.try_push(c)
    }

    #[inline]
    pub(crate) fn push_str(&mut self, s: &str) -> alloc::Result<()> {
        self.string.try_push_str(s)
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

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl TryWrite for Formatter {
    #[inline]
    fn try_write_str(&mut self, s: &str) -> alloc::Result<()> {
        self.string.try_push_str(s)
    }

    #[inline]
    fn try_write_char(&mut self, c: char) -> alloc::Result<()> {
        self.string.try_push(c)
    }
}
