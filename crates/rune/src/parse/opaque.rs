use crate::ast::Spanned;
use crate::parse::Id;

pub(crate) trait Opaque {
    /// Get an existing [Id] of an opaque element.
    fn id(&self) -> Option<Id>;
}

impl Opaque for Option<Id> {
    fn id(&self) -> Option<Id> {
        *self
    }
}

impl<T> Opaque for &T
where
    T: Opaque,
{
    fn id(&self) -> Option<Id> {
        Opaque::id(*self)
    }
}

impl<S> Opaque for (S, Id)
where
    S: Spanned,
{
    fn id(&self) -> Option<Id> {
        Some(self.1)
    }
}

impl<S> Opaque for (S, Option<Id>)
where
    S: Spanned,
{
    fn id(&self) -> Option<Id> {
        self.1
    }
}
