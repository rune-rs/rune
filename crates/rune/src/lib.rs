//! <div align="center">
//!     <a href="https://github.com/rune-rs/rune/actions">
//!         <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//!     </a>
//!
//!     <a href="https://github.com/rune-rs/rune/actions">
//!         <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
//!     </a>
//!
//!     <a href="https://discord.gg/v5AeNkT">
//!         <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//!     </a>
//! </div>
//!
//! An embeddable dynamic programming language for Rust.
//!
//! ## Contributing
//!
//! If you want to help out, there's a number of optimization tasks available in
//! [Future Optimizations][future-optimizations].
//!
//! Create an issue about the optimization you want to work on and communicate that
//! you are working on it.
//!
//! ## Features of runestick
//!
//! * [Clean Rust FFI][rust-ffi].
//! * Stack-based C FFI like with Lua (TBD).
//! * Stack frames, allowing for isolation across function calls.
//! * A rust-like reference language called *Rune*.
//!
//! ## Rune Scripts
//!
//! runestick comes with a simple scripting language called *Rune*.
//!
//! You can run example scripts through rune-cli:
//!
//! ```bash
//! cargo run -- ./scripts/hello_world.rn
//! ```
//!
//! If you want to see diagnostics of your unit, you can do:
//!
//! ```bash
//! cargo run -- ./scripts/hello_world.rn --dump-unit --trace
//! ```
//!
//! [rust-ffi]: https://github.com/rune-rs/rune/blob/master/crates/runestick-http/src/lib.rs
//! [future-optimizations]: https://github.com/rune-rs/rune/blob/master/FUTURE_OPTIMIZATIONS.md

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
pub use crate::error::{CompileError, Error, ParseError, Result};
pub use crate::lexer::Lexer;
pub use crate::parser::Parser;
#[cfg(feature = "runtime")]
pub use crate::runtime::{termcolor, Runtime};
pub use crate::source::Source;
pub use crate::token::{Kind, Token};
pub use crate::traits::{Parse, Resolve};
pub use runestick::unit::Span;
use runestick::Context;

/// Helper function to compile the given source.
///
/// Discards any warnings produced.
pub fn compile(context: &Context, source: &str) -> Result<(runestick::Unit, Warnings)> {
    let unit = parse_all::<ast::DeclFile>(&source)?;
    let (unit, warnings) = unit.compile(context)?;
    Ok((unit, warnings))
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

impl<'a, T> ParseAll<'a, T>
where
    T: Resolve<'a>,
{
    /// Resolve the item encapsulated in the parse.
    pub fn resolve(&self) -> Result<T::Output, ParseError> {
        self.item.resolve(self.source)
    }
}

/// Parse the given input as the given type that implements
/// [Parse][crate::traits::Parse].
///
/// This required the whole input to be parsed.
///
/// Returns the wrapped source and the parsed type.
pub fn parse_all<T>(source: &str) -> Result<ParseAll<T>, ParseError>
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
    pub use hashbrown::{hash_map, HashMap};
    pub use hashbrown::{hash_set, HashSet};
}
