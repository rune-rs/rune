use crate::alloc::{Box, Vec};
use crate::ast::Token;
use crate::compile::Result;
use crate::parse::{Parser, Peek};

/// Helper derive to implement [`Parse`].
pub use rune_macros::Parse;

/// Helper trait to convert a span and kind into an ast element.
pub trait ToAst
where
    Self: Sized,
{
    /// Coerce something into a primitive ast element.
    fn to_ast(token: Token) -> Result<Self>;
}

/// The parse trait, implemented by items that can be parsed.
pub trait Parse
where
    Self: Sized,
{
    /// Parse the current item from the parser.
    fn parse(p: &mut Parser<'_>) -> Result<Self>;
}

impl<A, B> Parse for (A, B)
where
    A: Parse + Peek,
    B: Parse,
{
    #[inline]
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        Ok((parser.parse()?, parser.parse()?))
    }
}

/// Parse implementation for something that can be optionally parsed.
impl<T> Parse for Option<T>
where
    T: Parse + Peek,
{
    #[inline]
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        Ok(if parser.peek::<T>()? {
            Some(parser.parse()?)
        } else {
            None
        })
    }
}

/// Parse implementation for something that is boxed.
impl<T> Parse for Box<T>
where
    T: Parse,
{
    #[inline]
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        Ok(Box::try_new(parser.parse()?)?)
    }
}

/// Parser implementation for a vector.
impl<T> Parse for Vec<T>
where
    T: Parse + Peek,
{
    #[inline]
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let mut output = Vec::new();

        while parser.peek::<T>()? {
            output.try_push(parser.parse()?)?;
        }

        Ok(output)
    }
}
