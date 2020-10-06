use std::fmt;
use std::num::NonZeroUsize;

/// An opaque identifier that is associated with AST items.
///
/// The default implementation for an identifier is empty, meaning it does not
/// hold any value, and attempting to perform lookups over it will fail with an
/// error indicating that it's empty with the string `Id(*)`.
///
/// This is used to store associated metadata to AST items through:
/// * [Query::insert_item](crate::Query::insert_item)
/// * [Query::insert_template](crate::Query::insert_template)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id(NonZeroUsize);

impl Id {
    /// Construct the initial (non-empty) id.
    pub fn initial() -> Self {
        Id(NonZeroUsize::new(1).unwrap())
    }

    /// Construct a new opaque identifier.
    pub fn new(index: usize) -> Option<Id> {
        NonZeroUsize::new(index).map(Self)
    }
}

impl Iterator for Id {
    type Item = Self;

    /// Return the next id based on the current. Returns `None` if the next ID
    /// could not be generated.
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.0.get().checked_add(1).and_then(NonZeroUsize::new)?;
        *self = Self(next);
        Some(*self)
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.0.get())
    }
}
