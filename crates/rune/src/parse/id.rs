use std::fmt;
use std::num::NonZeroU32;
use std::ops;

/// A non-zero [Id] which definitely contains a value. We keep this distinct
/// from `Id` to allow for safely using this as a key in a hashmap, preventing
/// us from inadvertently storing an empty identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NonZeroId(NonZeroU32);

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
#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Id(Option<NonZeroId>);

impl ops::Deref for Id {
    type Target = Option<NonZeroId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NonZeroId> for Id {
    fn from(value: NonZeroId) -> Self {
        Self(Some(value))
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
