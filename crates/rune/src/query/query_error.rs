use crate::compiling::{ImportEntryStep, InsertMetaError};
use crate::{
    Id, IrError, IrErrorKind, ParseError, ParseErrorKind, ResolveError, ResolveErrorKind, Spanned,
};
use runestick::{CompileMeta, Item, Location, Visibility};
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
}

/// Error raised during queries.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum QueryErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("failed to insert meta: {error}")]
    InsertMetaError {
        #[source]
        #[from]
        error: InsertMetaError,
    },
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
    #[error("missing {what} for id {id:?}")]
    MissingId { what: &'static str, id: Option<Id> },
    #[error("cannot define conflicting item `{item}`")]
    ItemConflict { item: Item, other: Location },
    #[error("`{item}` can refer to multiple things")]
    AmbiguousItem {
        item: Item,
        locations: Vec<(Location, Item)>,
    },
    #[error("`{item}` with {visibility} visibility, is not accessible from module `{from}`")]
    NotVisible {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: Item,
        from: Item,
    },
    #[error(
        "module `{item}` with {visibility} visibility, is not accessible from module `{from}`"
    )]
    NotVisibleMod {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: Item,
        from: Item,
    },
    #[error("missing item for id {id:?}")]
    MissingRevId { id: Id },
    #[error("missing query meta for module {item}")]
    MissingMod { item: Item },
    #[error("cycle in import")]
    ImportCycle { path: Vec<ImportEntryStep> },
    #[error("missing last use component")]
    LastUseComponent,
    #[error("found indexed entry for `{item}`, but was not an import")]
    NotIndexedImport { item: Item },
    #[error("{meta} can't be used as an import")]
    UnsupportedImportMeta { meta: CompileMeta },
    /// Tried to add an item that already exists.
    #[error("trying to insert `{current}` but conflicting meta `{existing}` already exists")]
    MetaConflict {
        /// The meta we tried to insert.
        current: CompileMeta,
        /// The existing item.
        existing: CompileMeta,
    },
}
