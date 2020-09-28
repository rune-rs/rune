use std::fmt;
use std::num::NonZeroUsize;
use std::ops;

/// An opaque identifier that is associated with AST items.
///
/// The default implementation for an identifier is empty, meaning it does not
/// hold any value, and attempting to perform lookups over it will fail with an
/// error indicating that it's empty with the string `Id(*)`.
///
/// This is used to store associated metadata to AST items through:
/// * [Query::insert_item](crate::Query::insert_item)
/// * [Query::insert_template](crate::Query::insert_template)
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id(Option<NonZeroUsize>);

impl Id {
    /// Construct a new identifier.
    pub(crate) fn new(index: usize) -> Id {
        Id(NonZeroUsize::new(index + 1))
    }
}

impl ops::Deref for Id {
    type Target = Option<NonZeroUsize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = self.0 {
            write!(f, "Id({})", index.get())
        } else {
            write!(f, "Id(*)")
        }
    }
}
