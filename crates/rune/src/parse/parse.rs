use crate::alloc::{Box, Vec};
use crate::compile;
use crate::parse::{Parser, Peek};

/// Helper derive to implement [`Parse`].
pub use rune_macros::Parse;

/// The parse trait, implemented by items that can be parsed.
pub trait Parse
where
    Self: Sized,
{
    /// Parse the current item from the parser.
    fn parse(p: &mut Parser) -> compile::Result<Self>;
}

impl<A, B> Parse for (A, B)
where
    A: Parse + Peek,
    B: Parse,
{
    #[inline]
    fn parse(parser: &mut Parser) -> compile::Result<Self> {
        Ok((parser.parse()?, parser.parse()?))
    }
}

/// Parse implementation for something that can be optionally parsed.
impl<T> Parse for Option<T>
where
    T: Parse + Peek,
{
    #[inline]
    fn parse(parser: &mut Parser) -> compile::Result<Self> {
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
    fn parse(parser: &mut Parser) -> compile::Result<Self> {
        Ok(Box::try_new(parser.parse()?)?)
    }
}

/// Parser implementation for a vector.
impl<T> Parse for Vec<T>
where
    T: Parse + Peek,
{
    #[inline]
    fn parse(parser: &mut Parser) -> compile::Result<Self> {
        let mut output = Vec::new();

        while parser.peek::<T>()? {
            output.try_push(parser.parse()?)?;
        }

        Ok(output)
    }
}
