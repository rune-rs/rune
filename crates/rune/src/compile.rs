//! The Rune compiler.
//!
//! The main entry to compiling rune source is [prepare][crate::prepare] which
//! uses this compiler. In here you'll just find compiler-specific types.

mod assembly;
pub(crate) use self::assembly::{Assembly, AssemblyInst};

pub(crate) mod attrs;

pub(crate) mod error;
pub use self::error::{Error, ImportStep};
pub(crate) use self::error::{ErrorKind, IrErrorKind};

mod compile_visitor;
pub use self::compile_visitor::CompileVisitor;
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

pub(crate) mod ir;

pub use rune_core::{Component, ComponentRef, IntoComponent, Item, ItemBuf};

mod source_loader;
pub use self::source_loader::{FileSourceLoader, NoopSourceLoader, SourceLoader};

mod unit_builder;
pub use self::unit_builder::LinkerError;
pub(crate) use self::unit_builder::UnitBuilder;

pub(crate) mod v1;

mod options;
pub use self::options::{Options, ParseOptionError};

mod location;
pub use self::location::Location;
pub(crate) use self::location::{DynLocation, Located};

pub mod meta;
pub(crate) use self::meta::{Doc, ItemMeta};
pub use self::meta::{MetaRef, SourceMeta};

mod pool;
pub(crate) use self::pool::{ItemId, ModId, ModMeta, Pool};

mod named;
pub use self::named::Named;

mod names;
pub(crate) use self::names::Names;

mod visibility;
pub(crate) use self::visibility::Visibility;

mod with_span;
pub use self::with_span::{HasSpan, WithSpan};

mod compile;
pub(crate) use self::compile::compile;

/// Helper alias for compile results.
pub type Result<T, E = Error> = ::core::result::Result<T, E>;
