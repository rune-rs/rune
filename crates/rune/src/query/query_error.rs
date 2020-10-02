use crate::compiling::{ImportEntryStep, InsertMetaError};
use crate::indexing::Visibility;
use crate::shared::Location;
use crate::{
    CompileError, CompileErrorKind, Id, IrError, IrErrorKind, ParseError, ParseErrorKind, Spanned,
};
use runestick::{CompileMeta, Item, Span};
use thiserror::Error;

error! {
    /// An error raised during querying.
    #[derive(Debug)]
    pub struct QueryError {
        kind: QueryErrorKind,
    }

    impl From<IrError>;
    impl From<CompileError>;
    impl From<ParseError>;
}

/// Error raised during queries.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum QueryErrorKind {
    #[error("internal error: {message}")]
    Internal { message: &'static str },
    #[error("failed to insert meta: {error}")]
    InsertMetaError {
        #[source]
        #[from]
        error: InsertMetaError,
    },
    #[error("interpreter error: {error}")]
    IrError {
        #[source]
        #[from]
        error: Box<IrErrorKind>,
    },
    #[error("compile error: {error}")]
    CompileError {
        #[source]
        #[from]
        error: Box<CompileErrorKind>,
    },
    #[error("parse error: {error}")]
    ParseError {
        #[source]
        #[from]
        error: ParseErrorKind,
    },
    #[error("missing {what} for id {id:?}")]
    MissingId { what: &'static str, id: Option<Id> },
    #[error("conflicting item `{item}`")]
    ItemConflict { item: Item, other: Location },
    #[error("item `{item}` with {visibility} visibility, is not accessible from here")]
    NotVisible {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: Item,
        from: Item,
    },
    #[error("module `{item}` with {visibility} visibility, is not accessible from here")]
    NotVisibleMod {
        chain: Vec<Location>,
        location: Location,
        visibility: Visibility,
        item: Item,
    },
    #[error("missing reverse lookup for `{item}`")]
    MissingRevItem { item: Item },
    #[error("missing item for id {id:?}")]
    MissingRevId { id: Id },
    #[error("missing query meta for module {item}")]
    MissingMod { item: Item },
    #[error("cycle in import")]
    ImportCycle { path: Vec<ImportEntryStep> },
    #[error("already imported `{item}`")]
    ImportConflict { item: Item, other: Location },
    #[error("missing last use component")]
    LastUseComponent,
    #[error("found indexed entry for `{item}`, but was not an import")]
    NotIndexedImport { item: Item },
    #[error("{meta} can't be used as an import")]
    UnsupportedImportMeta { meta: CompileMeta },
}
