//! Rune is a simple dynamic language for the ST virtual machine.

#![deny(missing_docs)]

pub mod ast;
mod compiler;
mod error;
mod lexer;
mod parser;
#[cfg(feature = "runtime")]
mod runtime;
mod source;
mod token;
mod traits;

pub use crate::compiler::{Options, Warning, Warnings};
pub use crate::error::{CompileError, Error, ParseError, ResolveError, Result, SpannedError};
pub use crate::lexer::Lexer;
pub use crate::parser::Parser;
#[cfg(feature = "runtime")]
pub use crate::runtime::{termcolor, Runtime};
pub use crate::source::Source;
pub use crate::token::{Kind, Token};
pub use crate::traits::Resolve;
pub use st::unit::Span;

/// Helper function to compile the given source.
///
/// Discards any warnings produced.
pub fn compile(source: &str) -> Result<st::CompilationUnit> {
    let unit = parse_all::<ast::File>(&source)?;
    let (unit, _) = unit.compile()?;
    Ok(unit)
}

/// The result from parsing a string.
pub struct ParseAll<'a, T> {
    /// The source parsed.
    ///
    /// Is needed to resolve things on the item through [Resolve::resolve]
    /// later.
    pub source: Source<'a>,
    /// The item parsed.
    pub item: T,
}

/// Parse the given input as the given type that implements
/// [Parse][crate::traits::Parse].
///
/// This required the whole input to be parsed.
///
/// Returns the wrapped source and the parsed type.
pub fn parse_all<'a, T>(source: &'a str) -> Result<ParseAll<T>, ParseError>
where
    T: crate::traits::Parse,
{
    let mut parser = Parser::new(source);
    let ast = parser.parse::<T>()?;

    if let Some(token) = parser.lexer.next()? {
        return Err(ParseError::ExpectedEof {
            actual: token.kind,
            span: token.span,
        });
    }

    Ok(ParseAll {
        source: Source { source },
        item: ast,
    })
}

mod collections {
    pub use std::collections::{hash_map, HashMap};
}
