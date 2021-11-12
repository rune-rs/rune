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
//! use rune::{Diagnostics, EmitDiagnostics, Context, Options, Sources, Vm, FromValue, Item, Source};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> rune::Result<()> {
//!     let context = Context::with_default_modules()?;
//!     let options = Options::default();
//!
//!     let mut sources = Sources::new();
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
//!     let mut diagnostics = Diagnostics::new();
//!
//!     let result = rune::load_sources(&context, &options, &mut sources, &mut diagnostics);
//!
//!     if !diagnostics.is_empty() {
//!         let mut writer = StandardStream::stderr(ColorChoice::Always);
//!         diagnostics.emit_diagnostics(&mut writer, &sources)?;
//!     }
//!
//!     let unit = result?;
//!     let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
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
#![allow(clippy::enum_variant_names)]
#![allow(clippy::needless_doctest_main)]
#![allow(clippy::never_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::result_unit_err)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::type_complexity)]

/// A macro that can be used to construct a [Span] that can be pattern matched
/// over.
///
/// # Examples
///
/// ```
/// let span = rune::Span::new(42, 84);
/// assert!(matches!(span, rune::span!(42, 84)));
/// ```
#[macro_export]
macro_rules! span {
    ($start:expr, $end:expr) => {
        $crate::Span {
            start: $crate::ByteIndex($start),
            end: $crate::ByteIndex($end),
        }
    };
}

/// Exported result type for convenience.
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

/// Exported boxed error type for convenience.
pub type Error = anyhow::Error;

#[macro_use]
mod internal_macros;
#[macro_use]
pub mod ast;

mod any;
pub use self::any::Any;

mod attrs;

pub mod compiling;

mod context;
pub use self::context::{Context, ContextError, ContextSignature, ContextTypeInfo};

pub mod diagnostics;
#[doc(inline)]
pub use self::diagnostics::Diagnostics;

#[cfg(feature = "diagnostics")]
mod emit_diagnostics;
#[cfg(feature = "diagnostics")]
#[doc(inline)]
pub use self::emit_diagnostics::{
    termcolor, DiagnosticsError, DumpInstructions, EmitDiagnostics, EmitSource,
};

mod hash;
pub use self::hash::{Hash, IntoTypeHash};

mod id;
pub use self::id::Id;

mod indexing;

pub mod ir;

mod item;
pub use self::item::{Component, ComponentRef, IntoComponent, Item};

mod load;
pub use self::load::{load_sources, load_sources_with_visitor, LoadSourcesError};

mod location;
pub use self::location::Location;

pub mod macros;

pub mod meta;

pub mod module;
#[doc(inline)]
pub use self::module::{InstFnNameHash, InstallWith, Module};

pub mod modules;

mod named;
pub use self::named::Named;

mod options;
pub use self::options::{ConfigurationError, Options};

pub mod parsing;

mod protocol;
pub use self::protocol::Protocol;

pub mod query;

mod raw_str;
pub use self::raw_str::RawStr;

pub mod runtime;
pub use self::runtime::{
    Args, BorrowMut, BorrowRef, FromValue, GuardedArgs, Mut, Panic, Ref, RuntimeContext, Shared,
    Stack, StackError, ToValue, Unit, UnsafeFromValue, UnsafeToValue, Value, Vm, VmError,
    VmErrorKind, VmExecution,
};

mod shared;

mod source;
pub use self::source::Source;

mod source_id;
pub use self::source_id::SourceId;

mod sources;
pub use self::sources::Sources;

mod span;
pub use self::span::{ByteIndex, IntoByteIndex, Span};

mod spanned;
pub use self::spanned::{OptionSpanned, Spanned};

mod spanned_error;
pub use self::spanned_error::SpannedError;
pub(crate) use self::spanned_error::WithSpan;

mod visibility;
pub(crate) use self::visibility::Visibility;

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
