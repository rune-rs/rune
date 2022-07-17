use crate::compile::{ImportStep, IrError, IrErrorKind, ItemBuf, Location, Meta, Visibility};
use crate::hir::{HirError, HirErrorKind};
use crate::parse::{Id, ParseError, ParseErrorKind, ResolveError, ResolveErrorKind};
use crate::runtime::debug::DebugSignature;
use crate::Hash;
use thiserror::Error;

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
pub enum QueryErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
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
    #[error("missing {what} for id {id:?}")]
    MissingId { what: &'static str, id: Id },
    #[error("cannot define conflicting item `{item}`")]
    ItemConflict { item: ItemBuf, other: Location },
    #[error("`{item}` can refer to multiple things")]
    AmbiguousItem {
        item: ItemBuf,
        locations: Vec<(Location, ItemBuf)>,
    },
    #[error("`{item}` with {visibility} visibility, is not accessible from module `{from}`")]
    NotVisible {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: ItemBuf,
        from: ItemBuf,
    },
    #[error(
        "module `{item}` with {visibility} visibility, is not accessible from module `{from}`"
    )]
    NotVisibleMod {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: ItemBuf,
        from: ItemBuf,
    },
    #[error("missing query meta for module {item}")]
    MissingMod { item: ItemBuf },
    #[error("cycle in import")]
    ImportCycle { path: Vec<ImportStep> },
    #[error("import recursion limit reached ({count})")]
    ImportRecursionLimit { count: usize, path: Vec<ImportStep> },
    #[error("missing last use component")]
    LastUseComponent,
    #[error("found indexed entry for `{item}`, but was not an import")]
    NotIndexedImport { item: ItemBuf },
    #[error("{meta} can't be used as an import")]
    UnsupportedImportMeta { meta: Meta },
    /// Tried to add an item that already exists.
    #[error("trying to insert `{current}` but conflicting meta `{existing}` already exists")]
    MetaConflict {
        /// The meta we tried to insert.
        current: Meta,
        /// The existing item.
        existing: Meta,
    },
    #[error("tried to insert rtti for conflicting variant with hash `{hash}`")]
    VariantRttiConflict { hash: Hash },
    #[error("tried to insert rtti for conflicting type with hash `{hash}`")]
    TypeRttiConflict { hash: Hash },
    #[error("conflicting function signature already exists `{existing}`")]
    FunctionConflict { existing: DebugSignature },
}
