use crate::ast::Spanned;
use crate::parse::Id;
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

impl<T> Opaque for &T
where
    T: Opaque,
{
    fn id(&self) -> Id {
        (*self).id()
    }
}

impl<S> Opaque for (S, Id)
where
    S: Spanned,
{
    fn id(&self) -> Id {
        self.1
    }
}
