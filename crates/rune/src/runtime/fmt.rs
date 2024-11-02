use core::ptr::NonNull;

use crate::alloc::fmt::TryWrite;
use crate::alloc::{self, String};
use crate::runtime::VmResult;
use crate::Any;

/// A formatter for the rune virtual machine.
///
/// This is used as a receiver to functions implementing the [`DEBUG_FMT`]
/// and [`DISPLAY_FMT`] protocols.
///
/// [`DEBUG_FMT`]: crate::runtime::Protocol::DEBUG_FMT
/// [`DISPLAY_FMT`]: crate::runtime::Protocol::DISPLAY_FMT
#[derive(Any)]
#[rune(crate, item = ::std::fmt)]
pub struct Formatter {
    pub(crate) out: NonNull<dyn TryWrite>,
    pub(crate) buf: String,
}

impl Formatter {
    /// Format onto the given trywrite.
    pub(crate) fn format_with(
        out: &mut String,
        f: impl FnOnce(&mut Self) -> VmResult<()>,
    ) -> VmResult<()> {
        // SAFETY: Call to this function ensures that the formatter does not
        // outlive the passed in reference.
        let mut fmt = Formatter {
            out: NonNull::from(out),
            buf: String::new(),
        };
        f(&mut fmt)
    }

    #[inline]
    pub(crate) fn parts_mut(&mut self) -> (&mut dyn TryWrite, &str) {
        // SAFETY: Formatter constrution requires `out` to be valid.
        (unsafe { self.out.as_mut() }, &self.buf)
    }

    #[inline]
    pub(crate) fn buf_mut(&mut self) -> &mut String {
        &mut self.buf
    }
}

impl TryWrite for Formatter {
    #[inline]
    fn try_write_str(&mut self, s: &str) -> alloc::Result<()> {
        // SAFETY: Formatter constrution requires `out` to be valid.
        unsafe { self.out.as_mut().try_write_str(s) }
    }

    #[inline]
    fn try_write_char(&mut self, c: char) -> alloc::Result<()> {
        // SAFETY: Formatter constrution requires `out` to be valid.
        unsafe { self.out.as_mut().try_write_char(c) }
    }
}
