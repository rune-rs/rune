//! Parsing utilities for Rune.

mod expectation;
mod id;
mod lexer;
mod opaque;
#[allow(clippy::module_inception)]
mod parse;
mod parse_error;
mod parser;
mod peek;
mod resolve;

pub use self::expectation::Expectation;
pub(crate) use self::expectation::IntoExpectation;
pub use self::id::{Id, NonZeroId};
pub(crate) use self::lexer::{Lexer, LexerMode};
pub(crate) use self::opaque::Opaque;
pub use self::parse::Parse;
pub use self::parse_error::{ParseError};
pub(crate) use self::parse_error::ParseErrorKind;
pub use self::parser::{Parser, Peeker};
pub use self::peek::Peek;
pub use self::resolve::{Resolve, ResolveContext, ResolveError};
pub(crate) use self::resolve::ResolveErrorKind;

use crate::SourceId;

/// Parse the given input as the given type that implements
/// [Parse][crate::parse::Parse]. The specified `source_id` will be used when
/// referencing any parsed elements. `shebang` indicates if the parser should
/// try to parse a shebang or not.
///
/// This will raise an error through [Parser::eof] if the specified `source` is
/// not fully consumed by the parser.
pub fn parse_all<T>(source: &str, source_id: SourceId, shebang: bool) -> Result<T, ParseError>
where
    T: Parse,
{
    let mut parser = Parser::new(source, source_id, shebang);
    let ast = parser.parse::<T>()?;
    parser.eof()?;
    Ok(ast)
}
