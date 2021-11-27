use std::fmt;
use std::num::NonZeroU32;

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
    pub(crate) fn as_ref(&self) -> Option<&NonZeroId> {
        self.0.as_ref()
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
