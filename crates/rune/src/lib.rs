//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/book/">
//!     <b>Read the book 📖</b>
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
//!     <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
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
//! * [Template literals 📖][support-templates].
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
//! cargo run --bin rune -- run scripts/hello_world.rn
//! ```
//!
//! If you want to see detailed diagnostics of your program while it's running,
//! you can use:
//!
//! ```text
//! cargo run --bin rune -- run scripts/hello_world.rn --dump-unit --trace --dump-vm
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
//!     let context = runestick::Context::with_default_modules()?;
//!     let options = rune::Options::default();
//!
//!     let mut sources = rune::Sources::new();
//!     sources.insert(Source::new(
//!         "script",
//!         r#"
//!         pub fn calculate(a, b) {
//!             println("Hello World");
//!             a + b
//!         }
//!         "#,
//!     ));
//!
//!     let mut diagnostics = rune::Diagnostics::new();
//!
//!     let result = rune::load_sources(&context, &options, &mut sources, &mut diagnostics);
//!
//!     if !diagnostics.is_empty() {
//!         let mut writer = StandardStream::stderr(ColorChoice::Always);
//!         diagnostics.emit_diagnostics(&mut writer, &sources)?;
//!     }
//!
//!     let unit = result?;
//!     let vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
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
//! [in the `examples` folder]: https://github.com/rune-rs/rune/tree/main/examples
//! [future-optimizations]: https://github.com/rune-rs/rune/blob/main/FUTURE_OPTIMIZATIONS.md
//! [Open Issues]: https://github.com/rune-rs/rune/issues
//! [support-rust-integration]: https://github.com/rune-rs/rune/tree/main/crates/rune-modules
//! [support-reference-counted]: https://rune-rs.github.io/book/variables.html
//! [support-templates]: https://rune-rs.github.io/book/template_literals.html
//! [support-try]: https://rune-rs.github.io/book/try_operator.html
//! [support-patterns]: https://rune-rs.github.io/book/pattern_matching.html
//! [support-structs]: https://rune-rs.github.io/book/structs.html
//! [support-async]: https://rune-rs.github.io/book/async.html
//! [support-generators]: https://rune-rs.github.io/book/generators.html
//! [support-instance-functions]: https://rune-rs.github.io/book/instance_functions.html
//! [support-stack-isolation]: https://rune-rs.github.io/book/call_frames.html
//! [support-dynamic-vectors]: https://rune-rs.github.io/book/vectors.html
//! [support-anon-objects]: https://rune-rs.github.io/book/objects.html
//! [support-anon-tuples]: https://rune-rs.github.io/book/tuples.html
//! [support-serde]: https://github.com/rune-rs/rune/blob/main/crates/rune-modules/src/json.rs

#![deny(missing_docs)]
#![allow(
    clippy::enum_variant_names,
    clippy::needless_doctest_main,
    clippy::never_loop,
    clippy::too_many_arguments,
    clippy::match_single_binding,
    clippy::unnecessary_lazy_evaluations,
    clippy::should_implement_trait,
    clippy::needless_borrow,
    clippy::len_zero,
    clippy::branches_sharing_code,
    clippy::needless_return,
    clippy::result_unit_err,
    clippy::useless_conversion,
    clippy::single_match,
    clippy::manual_map,
    clippy::collapsible_else_if,
    clippy::large_enum_variant,
    clippy::field_reassign_with_default,
    clippy::match_like_matches_macro,
    clippy::vec_init_then_push,
    clippy::collapsible_match
)]

#[macro_use]
mod internal_macros;
#[macro_use]
pub mod ast;
mod attrs;
mod compiling;
mod diagnostics;
#[cfg(feature = "diagnostics")]
mod emit_diagnostics;
mod indexing;
mod ir;
mod load;
pub mod macros;
mod options;
mod parsing;
mod query;
mod shared;
mod spanned;
mod worker;

#[doc(hidden)]
pub mod testing;

/// Internal collection re-export.
mod collections {
    pub use hashbrown::{hash_map, HashMap};
    pub use hashbrown::{hash_set, HashSet};
}

pub use self::compiling::{
    BuildError, CompileError, CompileErrorKind, CompileResult, CompileVisitor, ImportEntryStep,
    LinkerError, NoopCompileVisitor, UnitBuilder,
};
pub use self::diagnostics::{Diagnostic, Diagnostics, Error, ErrorKind, Warning, WarningKind};
#[cfg(feature = "diagnostics")]
pub use self::emit_diagnostics::{
    termcolor, DiagnosticsError, DumpInstructions, EmitDiagnostics, EmitSource,
};
pub use self::ir::{IrError, IrErrorKind, IrValue};
pub use self::load::{load_sources, load_sources_with_visitor, LoadSourcesError};
pub use self::load::{FileSourceLoader, SourceLoader, Sources};
pub use self::macros::{
    with_context, MacroContext, Quote, Storage, ToTokens, TokenStream, TokenStreamIter,
};
pub use self::options::{ConfigurationError, Options};
pub use self::parsing::{
    Id, Lexer, Parse, ParseError, ParseErrorKind, Parser, Peek, Peeker, Resolve, ResolveError,
    ResolveErrorKind, ResolveOwned,
};
pub use self::query::{QueryError, QueryErrorKind, Used};
pub use self::shared::{ScopeError, ScopeErrorKind};
pub use self::spanned::{OptionSpanned, Spanned};
pub use compiling::compile;
pub use rune_macros::quote;

pub(crate) use rune_macros::{OptionSpanned, Parse, Spanned, ToTokens};

/// Parse the given input as the given type that implements
/// [Parse][crate::parsing::Parse].
pub fn parse_all<T>(source: &str) -> Result<T, ParseError>
where
    T: crate::parsing::Parse,
{
    let mut parser = Parser::new(source);
    let ast = parser.parse::<T>()?;
    parser.eof()?;
    Ok(ast)
}
