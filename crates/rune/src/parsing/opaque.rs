use crate::Id;
use runestick::Span;

pub(crate) trait Opaque {
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
        Opaque::id(*self)
    }
}

impl Opaque for (Span, Id) {
    fn id(&self) -> Id {
        self.1
    }
}
