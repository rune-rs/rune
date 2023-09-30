use core::fmt;
use core::num::NonZeroU32;

use crate as rune;
use crate::alloc::prelude::*;

/// A non-zero [Id] which definitely contains a value. We keep this distinct
/// from `Id` to allow for safely using this as a key in a hashmap, preventing
/// us from inadvertently storing an empty identifier.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[repr(transparent)]
pub struct NonZeroId(#[try_clone(copy)] NonZeroU32);

impl fmt::Display for NonZeroId {
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

/// An opaque identifier that is associated with AST types.
///
/// The default implementation for an identifier is empty, meaning it does not
/// hold any value. Attempting to perform lookups over it will fail with an
/// error indicating that it's empty with the formatted string `Id(*)`.
#[derive(TryClone, Clone, Copy, Default, PartialEq, Eq)]
#[try_clone(copy)]
#[repr(transparent)]
pub struct Id(Option<NonZeroId>);

impl Id {
    /// Construct a new identifier with a value.
    pub(crate) const fn new(value: NonZeroId) -> Self {
        Self(Some(value))
    }

    /// Test if the identifier is set.
    pub(crate) fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Set the value of an identifier.
    pub(crate) fn set(&mut self, value: NonZeroId) {
        debug_assert!(self.0.is_none(), "id should not be set multiple times");
        self.0 = Some(value);
    }

    /// Get the underlying identifier.
    pub(crate) fn get(&self) -> Option<NonZeroId> {
        self.0
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(n) => write!(f, "Id({})", n.0.get()),
            None => write!(f, "Id(*)"),
        }
    }
}
