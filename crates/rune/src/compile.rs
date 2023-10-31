//! The Rune compiler.
//!
//! The main entry to compiling rune source is [prepare][crate::prepare] which
//! uses this compiler. In here you'll just find compiler-specific types.

pub use meta_info::MetaInfo;
pub use rune_core::{Component, ComponentRef, IntoComponent, Item, ItemBuf};

pub(crate) use self::assembly::{Assembly, AssemblyInst};
pub(crate) use self::compile::compile;
pub use self::compile_visitor::CompileVisitor;
#[cfg(feature = "std")]
pub(crate) use self::compile_visitor::NoopCompileVisitor;
pub use self::context::Context;
pub use self::context_error::ContextError;
pub(crate) use self::docs::Docs;
pub use self::error::{Error, ImportStep, MetaError};
pub(crate) use self::error::{ErrorKind, IrErrorKind};
pub(crate) use self::location::DynLocation;
pub use self::location::{Located, Location};
pub(crate) use self::meta::{Doc, ItemMeta};
pub use self::meta::{MetaRef, SourceMeta};
pub use self::named::Named;
pub(crate) use self::names::Names;
pub use self::options::{Options, ParseOptionError};
pub(crate) use self::pool::{ItemId, ModId, ModMeta, Pool};
pub(crate) use self::prelude::Prelude;
#[cfg(feature = "std")]
pub use self::source_loader::FileSourceLoader;
pub use self::source_loader::{NoopSourceLoader, SourceLoader};
pub use self::unit_builder::LinkerError;
pub(crate) use self::unit_builder::UnitBuilder;
pub(crate) use self::visibility::Visibility;
pub use self::with_span::{HasSpan, WithSpan};

mod assembly;
pub(crate) mod attrs;

mod compile_visitor;
pub(crate) mod context;
pub(crate) mod context_error;
mod docs;
pub(crate) mod error;
pub(crate) mod ir;
pub(crate) mod meta_info;
mod prelude;

mod source_loader;
mod unit_builder;
pub(crate) mod v1;

mod compile;
pub(crate) mod dynamic_fields;
mod location;
pub mod meta;
mod named;
mod names;
mod options;
mod pool;
mod visibility;
mod with_span;

/// Helper alias for compile results.
pub type Result<T, E = Error> = ::core::result::Result<T, E>;
