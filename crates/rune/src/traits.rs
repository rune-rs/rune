use crate::error::{ParseError, ResolveError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::Token;

/// The parse trait, implemented by items that can be parsed.
pub trait Parse
where
    Self: Sized,
{
    /// Parse the current item from the parser.
    fn parse(parser: &mut Parser) -> Result<Self, ParseError>;
}

/// Implemented by tokens that can be peeked for.
pub trait Peek {
    /// Peek the parser for the given token.
    fn peek(t1: Option<Token>, t2: Option<Token>) -> bool;
}

/// A type that can be resolved to an internal value based on a source.
pub trait Resolve<'a> {
    /// The output type being resolved.
    type Output: 'a;

    /// Resolve the value from parsed AST.
    fn resolve(&self, parsed: Source<'a>) -> Result<Self::Output, ResolveError>;
}
