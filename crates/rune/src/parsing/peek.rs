use crate::ast::Token;
use crate::parsing::Parse;

/// Implemented by tokens that can be peeked for.
pub trait Peek {
    /// Peek the parser for the given token.
    fn peek(t1: Option<Token>, t2: Option<Token>) -> bool;
}

/// Peek implementation for something that is boxed.
impl<T> Peek for Box<T>
where
    T: Peek,
{
    fn peek(t1: Option<Token>, t2: Option<Token>) -> bool {
        T::peek(t1, t2)
    }
}

impl<A, B> Peek for (A, B)
where
    A: Parse + Peek,
    B: Parse,
{
    fn peek(t1: Option<Token>, t2: Option<Token>) -> bool {
        A::peek(t1, t2)
    }
}
