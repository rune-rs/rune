use crate::ast::Spanned;
use crate::parse::{Id, NonZeroId};
pub(crate) use rune_macros::Opaque;

pub(crate) trait Opaque {
    /// Get an existing [Id] of an opaque element.
    fn id(&self) -> Id;
}

impl Opaque for Id {
    fn id(&self) -> Id {
        *self
    }
}

impl Opaque for NonZeroId {
    fn id(&self) -> Id {
        Id::new(*self)
    }
}

impl<T> Opaque for &T
where
    T: Opaque,
{
    fn id(&self) -> Id {
        (*self).id()
    }
}

impl<S, O> Opaque for (S, O)
where
    S: Spanned,
    O: Opaque,
{
    fn id(&self) -> Id {
        self.1.id()
    }
}
