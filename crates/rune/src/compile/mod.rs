//! The Rune compiler.
//!
//! The main entry to compiling rune source is [prepare][crate::prepare] which
//! uses this compiler. In here you'll just find compiler-specific types.

mod assembly;
pub(crate) use self::assembly::{Assembly, AssemblyInst};

pub(crate) mod error;
pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, ImportStep, MetaError};

mod compile_visitor;
pub use self::compile_visitor::CompileVisitor;
#[cfg(feature = "std")]
pub(crate) use self::compile_visitor::NoopCompileVisitor;

pub(crate) mod context;
pub use self::context::Context;

pub(crate) mod context_error;
pub use self::context_error::ContextError;

pub(crate) mod meta_info;
pub use meta_info::MetaInfo;

mod docs;
pub(crate) use self::docs::Docs;

mod prelude;
pub(crate) use self::prelude::Prelude;

pub(crate) mod const_eval;
pub(crate) use self::const_eval::ConstUnit;

mod source_loader;
#[cfg(feature = "std")]
pub use self::source_loader::FileSourceLoader;
pub use self::source_loader::{NoopSourceLoader, SourceLoader};

mod unit_builder;
pub use self::unit_builder::LinkerError;
pub(crate) use self::unit_builder::UnitBuilder;

pub(crate) mod v2;

mod options;
#[cfg(any(feature = "fmt", feature = "languageserver"))]
pub(crate) use self::options::{FmtOptions, IndentStyle};
pub use self::options::{Options, ParseOptionError};

mod location;
pub(crate) use self::location::DynLocation;
pub use self::location::{Located, Location};

pub mod meta;
pub(crate) use self::meta::{Doc, ItemMeta};
pub use self::meta::{MetaRef, SourceMeta};

mod pool;
pub use self::pool::ItemId;
pub(crate) use self::pool::{ModId, ModMeta, Pool};

mod named;
pub use self::named::Named;

mod names;
pub(crate) use self::names::Names;

pub(crate) mod num;

mod visibility;
pub(crate) use self::visibility::Visibility;

mod with_span;
pub(crate) use self::with_span::{HasSpan, WithSpan};

mod compile;
pub(crate) use self::compile::compile;

/// Helper alias for compile results.
pub type Result<T, E = Error> = core::result::Result<T, E>;
