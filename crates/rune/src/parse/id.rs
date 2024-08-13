use core::fmt;
use core::num::NonZeroU32;

use crate as rune;
use crate::alloc::prelude::*;

/// A non-zero identifier which definitely contains a value.
#[derive(TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[repr(transparent)]
pub(crate) struct NonZeroId(#[try_clone(copy)] NonZeroU32);

impl fmt::Display for NonZeroId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for NonZeroId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<NonZeroU32> for NonZeroId {
    fn from(value: NonZeroU32) -> Self {
        Self(value)
    }
}
