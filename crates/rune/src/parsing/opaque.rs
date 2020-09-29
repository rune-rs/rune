use crate::Id;
use runestick::Span;

pub(crate) trait Opaque {
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

impl Opaque for (Span, Option<Id>) {
    fn id(&self) -> Option<Id> {
        self.1
    }
}
