//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
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
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
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
//! ## Highlights of Rune
//!
//! * Clean [Rust integration 💻][support-rust-integration].
//! * Memory safe through [reference counting 📖][support-reference-counted].
//! * [Template strings 📖][support-templates].
//! * [Try operators 📖][support-try].
//! * [Pattern matching 📖][support-patterns].
//! * [Structs and enums 📖][support-structs] with associated data and functions.
//! * Dynamic [vectors 📖][support-dynamic-vectors], [objects 📖][support-anon-objects], and [tuples 📖][support-anon-tuples] with built-in [serde support 💻][support-serde].
//! * First-class [async support 📖][support-async].
//! * [Generators 📖][support-generators].
//! * Dynamic [instance functions 📖][support-instance-functions].
//! * [Stack isolation 📖][support-stack-isolation] between function calls.
//! * Stack-based C FFI, like Lua's (TBD).
//!
//! <br>
//!
//! ## Rune scripts
//!
//! You can run Rune programs with the bundled CLI:
//!
//! ```text
//! cargo run --bin rune -- scripts/hello_world.rn
//! ```
//!
//! If you want to see detailed diagnostics of your program while it's running,
//! you can use:
//!
//! ```text
//! cargo run --bin rune -- scripts/hello_world.rn --dump-unit --trace --dump-vm
//! ```
//!
//! See `--help` for more information.
//!
//! ## Running scripts from Rust
//!
//! > You can find more examples [in the `examples` folder].
//!
//! The following is a complete example, including rich diagnostics using
//! [`termcolor`]. It can be made much simpler if this is not needed.
//!
//! [`termcolor`]: https://docs.rs/termcolor
//!
//! ```rust
//! use rune::termcolor::{ColorChoice, StandardStream};
//! use rune::EmitDiagnostics as _;
//! use runestick::{Vm, FromValue as _, Item, Source};
//!
//! use std::error::Error;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     let context = Arc::new(rune::default_context()?);
//!     let options = rune::Options::default();
//!
//!     let mut sources = rune::Sources::new();
//!     sources.insert(Source::new(
//!         "script",
//!         r#"
//!         fn calculate(a, b) {
//!             println("Hello World");
//!             a + b
//!         }
//!         "#,
//!     ));
//!
//!     let mut errors = rune::Errors::new();
//!     let mut warnings = rune::Warnings::new();
//!
//!     let unit = match rune::load_sources(&*context, &options, &mut sources, &mut errors, &mut warnings) {
//!         Ok(unit) => unit,
//!         Err(rune::LoadSourcesError) => {
//!             let mut writer = StandardStream::stderr(ColorChoice::Always);
//!             errors.emit_diagnostics(&mut writer, &sources)?;
//!             return Ok(());
//!         }
//!     };
//!
//!     if !warnings.is_empty() {
//!         let mut writer = StandardStream::stderr(ColorChoice::Always);
//!         warnings.emit_diagnostics(&mut writer, &sources)?;
//!     }
//!
//!     let vm = Vm::new(context.clone(), Arc::new(unit));
//!
//!     let mut execution = vm.execute(&["calculate"], (10i64, 20i64))?;
//!     let value = execution.async_complete().await?;
//!
//!     let value = i64::from_value(value)?;
//!
//!     println!("{}", value);
//!     Ok(())
//! }
//! ```
//!
//! [in the `examples` folder]: https://github.com/rune-rs/rune/tree/master/crates/rune/examples
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
//! [support-stack-isolation]: https://rune-rs.github.io/rune/call_frames.html
//! [support-dynamic-vectors]: https://rune-rs.github.io/rune/vectors.html
//! [support-anon-objects]: https://rune-rs.github.io/rune/objects.html
//! [support-anon-tuples]: https://rune-rs.github.io/rune/tuples.html
//! [support-serde]: https://github.com/rune-rs/rune/blob/master/crates/rune-modules/src/json.rs

#![deny(missing_docs)]

#[macro_use]
mod util_macros;
mod assembly;
pub mod ast;
mod compile;
mod compile_error;
mod compile_visitor;
mod compiler;
#[cfg(feature = "diagnostics")]
pub mod diagnostics;
mod errors;
mod index;
mod index_scopes;
mod items;
mod lexer;
mod load;
mod load_error;
mod loops;
mod macro_context;
mod macros;
mod options;
mod parse_error;
mod parser;
mod query;
mod quote;
mod scopes;
mod source_loader;
mod sources;
mod storage;
mod token_stream;
mod traits;
mod unit_builder;
mod warning;
mod worker;
// NB: this has to be defined before the `tests` module, because it's used in
// there.
#[cfg(any(test, feature = "testing"))]
#[macro_use]
pub mod testing;
#[cfg(test)]
pub mod tests;

/// Internal collection re-export.
mod collections {
    pub use hashbrown::{hash_map, HashMap};
    pub use hashbrown::{hash_set, HashSet};
}

pub use crate::assembly::Assembly;
pub use crate::compile_error::{CompileError, CompileErrorKind, CompileResult};
pub use crate::compile_visitor::{CompileVisitor, NoopCompileVisitor};
pub use crate::errors::Errors;
pub use crate::lexer::Lexer;
pub use crate::load::{load_sources, load_sources_with_visitor, LoadSourcesError};
pub use crate::load_error::{LoadError, LoadErrorKind};
pub use crate::macro_context::MacroContext;
pub use crate::options::Options;
pub use crate::parse_error::{ParseError, ParseErrorKind};
pub use crate::parser::Parser;
pub use crate::scopes::Var;
pub use crate::source_loader::{FileSourceLoader, SourceLoader};
pub use crate::sources::Sources;
pub use crate::storage::Storage;
pub use crate::token_stream::{IntoTokens, TokenStream, TokenStreamIter};
pub use crate::traits::{Parse, Peek, Resolve, Spanned};
pub use crate::warning::{Warning, WarningKind, Warnings};
pub use compiler::compile;
pub use unit_builder::{ImportEntry, ImportKey, LinkerError, UnitBuilder};

#[cfg(feature = "diagnostics")]
pub use diagnostics::{termcolor, DiagnosticsError, EmitDiagnostics};

/// Construct a a default context runestick context.
///
/// If built with the `modules` feature, this includes all available native
/// modules.
///
/// See [load_sources](crate::load_sources) for how to use.
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
    parser.parse_eof()?;
    Ok(ast)
}
