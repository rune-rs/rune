use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::compile::{ImportStep, IrError, IrErrorKind, ItemBuf, Location, MetaInfo, Visibility};
use crate::hir::{HirError, HirErrorKind};
use crate::parse::{Id, ParseError, ParseErrorKind, ResolveError, ResolveErrorKind};
use crate::runtime::debug::DebugSignature;
use crate::Hash;

error! {
    /// An error raised during querying.
    #[derive(Debug)]
    pub struct QueryError {
        kind: QueryErrorKind,
    }

    impl From<IrError>;
    impl From<ParseError>;
    impl From<ResolveError>;
    impl From<HirError>;
}

/// Error raised during queries.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum QueryErrorKind {
    #[error("{message}")]
    Custom { message: Box<str> },
    #[error("{error}")]
    IrError {
        #[source]
        #[from]
        error: IrErrorKind,
    },
    #[error("{error}")]
    ParseError {
        #[source]
        #[from]
        error: ParseErrorKind,
    },
    #[error("{error}")]
    ResolveError {
        #[source]
        #[from]
        error: ResolveErrorKind,
    },
    #[error("{error}")]
    HirError {
        #[source]
        #[from]
        error: HirErrorKind,
    },
    #[error("Missing {what} for id {id:?}")]
    MissingId { what: &'static str, id: Id },
    #[error("Item `{item}` can refer to multiple things")]
    AmbiguousItem {
        item: ItemBuf,
        locations: Vec<(Location, ItemBuf)>,
    },
    #[error(
        "Item `{item}` with visibility `{visibility}`, is not accessible from module `{from}`"
    )]
    NotVisible {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: ItemBuf,
        from: ItemBuf,
    },
    #[error(
        "Module `{item}` with {visibility} visibility, is not accessible from module `{from}`"
    )]
    NotVisibleMod {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: ItemBuf,
        from: ItemBuf,
    },
    #[error("Missing query meta for module {item}")]
    MissingMod { item: ItemBuf },
    #[error("Cycle in import")]
    ImportCycle { path: Vec<ImportStep> },
    #[error("Import recursion limit reached ({count})")]
    ImportRecursionLimit { count: usize, path: Vec<ImportStep> },
    #[error("Missing last use component")]
    LastUseComponent,
    /// Tried to add an item that already exists.
    #[error("Item `{current}` but conflicting meta `{existing}` already exists")]
    MetaConflict {
        /// The meta we tried to insert.
        current: MetaInfo,
        /// The existing item.
        existing: MetaInfo,
    },
    #[error("Tried to insert variant runtime type information, but conflicted with hash `{hash}`")]
    VariantRttiConflict { hash: Hash },
    #[error("Tried to insert runtime type information, but conflicted with hash `{hash}`")]
    TypeRttiConflict { hash: Hash },
    #[error("Conflicting function signature already exists `{existing}`")]
    FunctionConflict { existing: DebugSignature },
}
