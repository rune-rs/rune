use core::convert::Infallible;
use core::fmt;

use rune::alloc::fmt::TryWrite;
use rune::runtime::{Formatter, VmResult};
use rune::{vm_write, Any};

/// An error returned by fallible `try_from_rng` methods.
#[derive(Any)]
#[rune(item = ::rand)]
pub struct TryFromRngError {
    kind: TryFromRngErrorKind,
}

impl TryFromRngError {
    /// Write a display representation the error.
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{}", self.kind)
    }
}

#[cfg(feature = "os_rng")]
impl From<rand::rand_core::OsError> for TryFromRngError {
    #[inline]
    fn from(inner: rand::rand_core::OsError) -> Self {
        Self {
            kind: TryFromRngErrorKind::OsError(inner),
        }
    }
}

impl From<Infallible> for TryFromRngError {
    #[inline]
    fn from(inner: Infallible) -> Self {
        match inner {}
    }
}

impl fmt::Debug for TryFromRngError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug)]
enum TryFromRngErrorKind {
    #[cfg(feature = "os_rng")]
    OsError(rand::rand_core::OsError),
}

impl fmt::Display for TryFromRngErrorKind {
    #[inline]
    fn fmt(&self, #[allow(unused)] f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            #[cfg(feature = "os_rng")]
            TryFromRngErrorKind::OsError(ref inner) => {
                write!(f, "{inner}")
            }
        }
    }
}
