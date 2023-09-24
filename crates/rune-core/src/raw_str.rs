use core::fmt;
use core::ops;
use core::slice;
use core::str;

use crate::alloc;
use crate::alloc::clone::TryClone;

/// A raw static string.
///
/// We define and use this instead of relying on `&'static str` (which should
/// have a similar layout) because we want to allow static construction of the
/// `RawStr` through a C-ffi.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RawStr {
    data: *const u8,
    len: usize,
}

impl RawStr {
    /// Construct from a static string.
    pub const fn from_str(s: &'static str) -> Self {
        Self {
            data: s.as_ptr(),
            len: s.len(),
        }
    }
}

impl TryClone for RawStr {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(*self)
    }
}

impl From<&'static str> for RawStr {
    fn from(s: &'static str) -> Self {
        Self::from_str(s)
    }
}

/// RawStr derefs into a str.
impl ops::Deref for RawStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.data, self.len)) }
    }
}

impl fmt::Debug for RawStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl fmt::Display for RawStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl PartialEq for RawStr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Eq for RawStr {}

// Safety: `RawStr` references static data.
unsafe impl Send for RawStr {}
unsafe impl Sync for RawStr {}
