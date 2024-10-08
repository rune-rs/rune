//! Parsing utilities for Rune.

mod expectation;
mod id;
mod lexer;
mod parse;
mod parser;
mod peek;
mod resolve;
mod traits;

pub use self::expectation::Expectation;
pub(crate) use self::expectation::IntoExpectation;
pub(crate) use self::id::NonZeroId;
pub(crate) use self::lexer::{Lexer, LexerMode};
pub use self::parse::Parse;
pub use self::parser::{Parser, Peeker};
pub use self::peek::Peek;
pub(crate) use self::resolve::{Resolve, ResolveContext};
pub(crate) use self::traits::Advance;

use crate::compile;
use crate::SourceId;

/// Parse the given input as the given type that implements [Parse]. The
/// specified `source_id` will be used when referencing any parsed elements.
/// `shebang` indicates if the parser should try to parse a shebang or not.
///
/// This will raise an error through [Parser::eof] if the specified `source` is
/// not fully consumed by the parser.
pub fn parse_all<T>(source: &str, source_id: SourceId, shebang: bool) -> compile::Result<T>
where
    T: Parse,
{
    let mut parser = Parser::new(source, source_id, shebang);
    let ast = parser.parse::<T>()?;
    parser.eof()?;
    Ok(ast)
}
