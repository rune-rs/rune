use crate::alloc::Box;
use crate::parse::{Parse, Peeker};

/// Implemented by tokens that can be peeked for.
pub trait Peek {
    /// Peek the parser for the given token.
    fn peek(p: &mut Peeker<'_>) -> bool;
}

/// Peek implementation for something that is boxed.
impl<T> Peek for Box<T>
where
    T: Peek,
{
    #[inline]
    fn peek(p: &mut Peeker<'_>) -> bool {
        T::peek(p)
    }
}

impl<A, B> Peek for (A, B)
where
    A: Parse + Peek,
    B: Parse,
{
    #[inline]
    fn peek(p: &mut Peeker<'_>) -> bool {
        A::peek(p)
    }
}
