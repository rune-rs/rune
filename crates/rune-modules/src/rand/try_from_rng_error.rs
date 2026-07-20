use core::convert::Infallible;
use core::fmt;

use rune::alloc;
use rune::alloc::fmt::TryWrite;
use rune::runtime::Formatter;
use rune::Any;

/// An error returned by fallible `try_from_rng` methods.
#[derive(Any)]
#[rune(item = ::rand)]
pub struct TryFromRngError {
    kind: TryFromRngErrorKind,
}

impl TryFromRngError {
    /// Write a display representation the error.
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{}", self.kind)
    }
}

#[cfg(feature = "sys_rng")]
impl From<rand::rngs::SysError> for TryFromRngError {
    #[inline]
    fn from(inner: rand::rngs::SysError) -> Self {
        Self {
            kind: TryFromRngErrorKind::SysError(inner),
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
    #[cfg(feature = "sys_rng")]
    SysError(rand::rngs::SysError),
}

impl fmt::Display for TryFromRngErrorKind {
    #[inline]
    fn fmt(&self, #[allow(unused)] f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            #[cfg(feature = "sys_rng")]
            TryFromRngErrorKind::SysError(ref inner) => {
                write!(f, "{inner}")
            }
        }
    }
}
