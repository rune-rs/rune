//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! <br>
//!
//! An embeddable dynamic programming language for Rust.
//!
//! ## Contributing
//!
//! If you want to help out, there should be a number of optimization tasks
//! available in [Future Optimizations][future-optimizations]. Or have a look at
//! [Open Issues].
//!
//! Create an issue about the optimization you want to work on and communicate that
//! you are working on it.
//!
//! <br>
//!
//! ## Features of Rune
//!
//! * Clean [Rust Integration 💻][support-rust-integration].
//! * Memory safe through [reference counting 📖][support-reference-counted].
//! * [Template strings 📖][support-templates].
//! * [Try operators 📖][support-try].
//! * Pattern matching [📖][support-patterns].
//! * [Structs and enums 📖][support-structs] with associated data and functions.
//! * Dynamic [vectors 📖][support-dynamic-vectors], [objects 📖][support-anon-objects], and [tuples 📖][support-anon-tuples] with built-in [serde support 💻][support-serde].
//! * First-class [async support 📖][support-async].
//! * [Generators 📖][support-generators].
//! * Dynamic [instance functions 📖][support-instance-functions].
//! * Stack isolation between function calls.
//! * Stack-based C FFI, like Lua's (TBD).
//!
//! <br>
//!
//! ## Rune Scripts
//!
//! You can run Rune programs with the bundled CLI:
//!
//! ```text
//! cargo run -- scripts/hello_world.rn
//! ```
//!
//! If you want to see detailed diagnostics of your program while it's running,
//! you can use:
//!
//! ```text
//! cargo run -- scripts/hello_world.rn --dump-unit --trace --dump-vm
//! ```
//!
//! See `--help` for more information.
//!
//! [future-optimizations]: https://github.com/rune-rs/rune/blob/master/FUTURE_OPTIMIZATIONS.md
//! [Open Issues]: https://github.com/rune-rs/rune/issues
//! [support-rust-integration]: https://github.com/rune-rs/rune/tree/master/crates/rune-modules
//! [support-reference-counted]: https://rune-rs.github.io/rune/variables.html
//! [support-templates]: https://rune-rs.github.io/rune/template_strings.html
//! [support-try]: https://rune-rs.github.io/rune/try_operator.html
//! [support-patterns]: https://rune-rs.github.io/rune/pattern_matching.html
//! [support-structs]: https://rune-rs.github.io/rune/structs.html
//! [support-async]: https://rune-rs.github.io/rune/async.html
//! [support-generators]: https://rune-rs.github.io/rune/generators.html
//! [support-instance-functions]: https://rune-rs.github.io/rune/instance_functions.html
//! [support-dynamic-vectors]: https://rune-rs.github.io/rune/vectors.html
//! [support-anon-objects]: https://rune-rs.github.io/rune/objects.html
//! [support-anon-tuples]: https://rune-rs.github.io/rune/tuples.html
//! [support-serde]: https://github.com/rune-rs/rune/blob/master/crates/rune-modules/src/json.rs

#![deny(missing_docs)]

pub mod ast;
mod compile;
mod compiler;
#[cfg(feature = "diagnostics")]
mod diagnostics;
mod error;
mod index;
mod index_scopes;
mod items;
mod lexer;
mod load;
mod load_error;
mod loops;
mod options;
mod parser;
mod query;
mod scopes;
mod traits;
mod warning;

/// Internal collection re-export.
mod collections {
    pub use hashbrown::{hash_map, HashMap};
    pub use hashbrown::{hash_set, HashSet};
}

pub use crate::error::{CompileError, ParseError};
pub use crate::lexer::Lexer;
pub use crate::load::{load_path, load_source};
pub use crate::load_error::{LoadError, LoadErrorKind};
pub use crate::options::Options;
pub use crate::parser::Parser;
pub use crate::warning::{Warning, WarningKind, Warnings};
pub use compiler::compile;

#[cfg(feature = "diagnostics")]
pub use diagnostics::{emit_warning_diagnostics, termcolor, DiagnosticsError, EmitDiagnostics};

/// Construct a a default context runestick context.
///
/// If built with the `modules` feature, this includes all available native
/// modules.
///
/// See [load_path](crate::load_path) for how to use.
pub fn default_context() -> Result<runestick::Context, runestick::ContextError> {
    #[allow(unused_mut)]
    let mut context = runestick::Context::with_default_modules()?;

    #[cfg(feature = "modules")]
    {
        context.install(&rune_modules::http::module()?)?;
        context.install(&rune_modules::json::module()?)?;
        context.install(&rune_modules::toml::module()?)?;
        context.install(&rune_modules::time::module()?)?;
        context.install(&rune_modules::process::module()?)?;
        context.install(&rune_modules::fs::module()?)?;
        context.install(&rune_modules::signal::module()?)?;
    }

    Ok(context)
}

/// Parse the given input as the given type that implements
/// [Parse][crate::traits::Parse].
pub fn parse_all<T>(source: &str) -> Result<T, ParseError>
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

    Ok(ast)
}
