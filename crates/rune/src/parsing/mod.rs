//! Parsing utilities for Rune.

mod lexer;
mod opaque;
mod parse;
mod parse_error;
mod parser;
mod peek;
mod resolve;

pub use self::lexer::{Lexer, LexerMode};
pub(crate) use self::opaque::Opaque;
pub use self::parse::Parse;
pub use self::parse_error::{ParseError, ParseErrorKind};
pub use self::parser::{Parser, Peeker};
pub use self::peek::Peek;
pub use self::resolve::{Resolve, ResolveError, ResolveErrorKind};

use crate::SourceId;

/// Parse the given input as the given type that implements
/// [Parse][crate::parsing::Parse]. The specified `source_id` will be used when
/// referencing any parsed elements.
pub fn parse_all<T>(source: &str, source_id: SourceId) -> Result<T, ParseError>
where
    T: Parse,
{
    let mut parser = Parser::new(source, source_id);
    let ast = parser.parse::<T>()?;
    parser.eof()?;
    Ok(ast)
}
