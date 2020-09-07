use crate::ast::Token;
use crate::error::CompileResult;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::Storage;
use runestick::Source;

/// The parse trait, implemented by items that can be parsed.
pub trait Parse
where
    Self: Sized,
{
    /// Parse the current item from the parser.
    fn parse(parser: &mut Parser) -> Result<Self, ParseError>;
}

impl<A, B> Parse for (A, B)
where
    A: Parse + Peek,
    B: Parse,
{
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok((parser.parse()?, parser.parse()?))
    }
}

/// Parse implementation for something that can be optionally parsed.
impl<T> Parse for Option<T>
where
    T: Parse + Peek,
{
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
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
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Box::new(parser.parse()?))
    }
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

/// Parser implementation for a vector.
impl<T> Parse for Vec<T>
where
    T: Parse + Peek,
{
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let mut output = Vec::new();

        while parser.peek::<T>()? {
            output.push(parser.parse()?);
        }

        Ok(output)
    }
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
    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError>;
}

pub(crate) trait Compile<T> {
    /// Walk the current type with the given item.
    fn compile(&mut self, item: T) -> CompileResult<()>;
}
