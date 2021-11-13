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
//! use rune::{Context, Diagnostics, FromValue, Source, Sources, Vm};
//! use rune::termcolor::{ColorChoice, StandardStream};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> rune::Result<()> {
//!     let context = Context::with_default_modules()?;
//!     let runtime = Arc::new(context.runtime());
//!
//!     let mut sources = Sources::new();
//!     sources.insert(Source::new(
//!         "script",
//!         r#"
//!         pub fn add(a, b) {
//!             a + b
//!         }
//!         "#,
//!     ));
//!
//!     let mut diagnostics = Diagnostics::new();
//!
//!     let result = rune::prepare(&mut sources)
//!         .with_context(&context)
//!         .with_diagnostics(&mut diagnostics)
//!         .build();
//!
//!     if !diagnostics.is_empty() {
//!         let mut writer = StandardStream::stderr(ColorChoice::Always);
//!         diagnostics.emit(&mut writer, &sources)?;
//!     }
//!
//!     let unit = result?;
//!     let mut vm = Vm::new(runtime, Arc::new(unit));
//!
//!     let output = vm.call(&["add"], (10i64, 20i64))?;
//!     let output = i64::from_value(output)?;
//!
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! [in the `examples` folder]: https://github.com/rune-rs/rune/tree/main/examples/examples
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
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::needless_doctest_main)]
#![allow(clippy::never_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::result_unit_err)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::type_complexity)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// A macro that can be used to construct a [Span][crate::ast::Span] that can be
/// pattern matched over.
///
/// # Examples
///
/// ```
/// use rune::ast::Span;
/// use rune::span;
///
/// let span = Span::new(42, 84);
/// assert!(matches!(span, span!(42, 84)));
/// ```
#[macro_export]
macro_rules! span {
    ($start:expr, $end:expr) => {
        $crate::ast::Span {
            start: $crate::ast::ByteIndex($start),
            end: $crate::ast::ByteIndex($end),
        }
    };
}

/// Exported result type for convenience using [anyhow::Error] as the default
/// error type.
pub type Result<T, E = anyhow::Error> = ::std::result::Result<T, E>;

/// Boxed error type, which is an alias of [anyhow::Error].
pub type Error = anyhow::Error;

#[macro_use]
mod internal_macros;

#[macro_use]
pub mod ast;

cfg_emit! {
    pub use ::codespan_reporting::term::termcolor;
}

mod any;
pub use self::any::Any;

mod attrs;

mod build;
pub use self::build::{prepare, Build, BuildError};

pub mod compile;
#[doc(inline)]
pub use self::compile::{Context, ContextError, InstallWith, Module, Options};

pub mod diagnostics;
#[doc(inline)]
pub use self::diagnostics::Diagnostics;

mod hash;
pub use self::hash::{Hash, InstFnNameHash, IntoTypeHash};

mod indexing;

pub mod ir;

pub mod macros;

pub mod meta;

pub mod modules;

pub mod parse;

pub mod query;

pub mod runtime;
pub use self::runtime::{FromValue, Panic, Protocol, Stack, ToValue, Unit, Value, Vm};

mod shared;

mod source;
pub use self::source::Source;

mod source_id;
pub use self::source_id::SourceId;

#[macro_use]
mod sources;
pub use self::sources::Sources;

mod worker;

#[doc(hidden)]
pub mod testing;

// Macros used internally and re-exported.
pub(crate) use rune_macros::__internal_impl_any;

/// Internal collection re-export.
mod collections {
    pub use hashbrown::{hash_map, HashMap};
    pub use hashbrown::{hash_set, HashSet};
    pub use std::collections::{btree_map, BTreeMap};
}
