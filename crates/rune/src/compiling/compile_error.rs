use crate::ast;
use crate::compiling::{InsertMetaError, UnitBuilderError, UnitBuilderErrorKind};
use crate::{
    IrError, IrErrorKind, ParseError, ParseErrorKind, QueryError, QueryErrorKind, Spanned,
};
use runestick::{CompileMeta, Item, SourceId, Span};
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// A compile result.
pub type CompileResult<T> = std::result::Result<T, CompileError>;

error! {
    /// An error raised during compiling.
    #[derive(Debug)]
    pub struct CompileError {
        kind: CompileErrorKind,
    }

    impl From<ParseError>;
    impl From<IrError>;
    impl From<QueryError>;
    impl From<UnitBuilderError>;
}

impl CompileError {
    /// Construct an internal error.
    ///
    /// This should be used for programming invariants of the encoder which are
    /// broken for some reason.
    pub fn internal<S>(spanned: S, msg: &'static str) -> Self
    where
        S: Spanned,
    {
        CompileError::new(spanned, CompileErrorKind::Internal { msg })
    }

    /// Construct an "unsupported path" internal error for the
    /// paths containing unsupported path keywords like super and crate.
    pub fn internal_unsupported_path<S>(spanned: S) -> Self
    where
        S: Spanned,
    {
        CompileError::new(
            spanned,
            CompileErrorKind::Internal {
                msg: "paths containing `crate` or `super` are not supported",
            },
        )
    }

    /// An error raised during constant computation.
    pub fn const_error<S>(spanned: S, msg: &'static str) -> Self
    where
        S: Spanned,
    {
        CompileError::new(spanned, CompileErrorKind::ConstError { msg })
    }

    /// Construct an experimental error.
    ///
    /// This should be used when an experimental feature is used which hasn't
    /// been enabled.
    pub fn experimental<S>(spanned: S, msg: &'static str) -> Self
    where
        S: Spanned,
    {
        Self::new(spanned, CompileErrorKind::Experimental { msg })
    }
}

/// Compiler error.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum CompileErrorKind {
    #[error("internal compiler error: {msg}")]
    Internal { msg: &'static str },
    #[error("ir error: {error}")]
    IrError {
        #[source]
        error: Box<IrErrorKind>,
    },
    #[error("query error: {error}")]
    QueryError {
        #[source]
        error: Box<QueryErrorKind>,
    },
    #[error("unit construction error: {error}")]
    UnitBuilderError {
        #[source]
        #[from]
        error: UnitBuilderErrorKind,
    },
    #[error("{error}")]
    ParseError {
        #[source]
        error: ParseErrorKind,
    },
    #[error("failed to insert meta: {error}")]
    InsertMetaError {
        #[source]
        #[from]
        error: InsertMetaError,
    },
    #[error("error during constant evaluation: {msg}")]
    ConstError { msg: &'static str },
    #[error("experimental feature: {msg}")]
    Experimental { msg: &'static str },
    #[error("file not found, expected a module file like `{path}.rn`")]
    ModNotFound { path: PathBuf },
    #[error("failed to load `{path}`: {error}")]
    ModFileError {
        path: PathBuf,
        #[source]
        error: io::Error,
    },
    #[error("module `{item}` has already been loaded")]
    ModAlreadyLoaded {
        item: Item,
        existing: (SourceId, Span),
    },
    #[error("found conflicting item `{existing}`")]
    ItemConflict { existing: Item },
    #[error("variable `{name}` conflicts")]
    VariableConflict { name: String, existing_span: Span },
    #[error("missing macro `{item}`")]
    MissingMacro { item: Item },
    #[error("error while calling macro: {error}")]
    CallMacroError { error: runestick::Error },
    #[error("no local variable `{name}`")]
    MissingLocal { name: String },
    #[error("no such type `{item}`")]
    MissingType { item: Item },
    #[error("missing module `{item}`")]
    MissingModule { item: Item },
    #[error("label not found in scope")]
    MissingLabel,
    #[error("cannot load modules using a source without an associated URL")]
    UnsupportedModuleSource,
    #[error("cannot load modules relative to `{root}`")]
    UnsupportedModuleRoot { root: PathBuf },
    #[error("cannot load module for `{item}`")]
    UnsupportedModuleItem { item: Item },
    #[error("wildcard support not supported in this position")]
    UnsupportedWildcard,
    #[error("`{meta}` is not a supported async block")]
    UnsupportedAsyncBlock { meta: CompileMeta },
    #[error("cannot declare instance functions for type `{meta}`")]
    UnsupportedInstanceFunction { meta: CompileMeta },
    #[error("`{meta}` cannot be used as a value")]
    UnsupportedValue { meta: CompileMeta },
    #[error("`{meta}` cannot be used as a type")]
    UnsupportedType { meta: CompileMeta },
    #[error("`self` not supported here")]
    UnsupportedSelf,
    #[error("unsupported unary operator `{op}`")]
    UnsupportedUnaryOp { op: ast::UnaryOp },
    #[error("unsupported binary operator `{op}`")]
    UnsupportedBinaryOp { op: ast::BinOp },
    #[error("type `{item}` is not an object")]
    UnsupportedLitObject { item: Item },
    #[error("missing field `{field}` in declaration of `{item}`")]
    LitObjectMissingField { field: String, item: Item },
    #[error("`{field}` is not a field in `{item}`")]
    LitObjectNotField { field: String, item: Item },
    #[error("cannot assign to expression")]
    UnsupportedAssignExpr,
    #[error("unsupported binary expression")]
    UnsupportedBinaryExpr,
    #[error("cannot take reference of expression")]
    UnsupportedRef,
    #[error("unsupported select pattern")]
    UnsupportedSelectPattern,
    #[error("unsupported field access")]
    UnsupportedFieldAccess,
    #[error("wrong number of arguments, expected `{expected}` but got `{actual}`")]
    UnsupportedArgumentCount {
        meta: CompileMeta,
        expected: usize,
        actual: usize,
    },
    #[error("`{meta}` cannot be used as a const")]
    UnsupportedMetaConst { meta: CompileMeta },
    #[error("`{meta}` is not supported in a pattern like this")]
    UnsupportedMetaPattern { meta: CompileMeta },
    #[error("`{meta}` is not supported as a closure")]
    UnsupportedMetaClosure { meta: CompileMeta },
    #[error("item is not supported in a pattern")]
    UnsupportedPattern,
    #[error("not a valid binding")]
    UnsupportedBinding,
    #[error("floating point numbers cannot be used in patterns")]
    MatchFloatInPattern,
    #[error("duplicate key in literal object")]
    DuplicateObjectKey { existing: Span, object: Span },
    #[error("`{item}` is not a function")]
    MissingFunction { item: Item },
    #[error("`yield` must be used in function or closure")]
    YieldOutsideFunction,
    #[error("`await` must be used inside an async function or closure")]
    AwaitOutsideFunction,
    #[error("instance function declared outside of `impl` block")]
    InstanceFunctionOutsideImpl,
    #[error("import `{item}` (imported in prelude) does not exist")]
    MissingPreludeModule { item: Item },
    #[error("unsupported tuple index `{number}`")]
    UnsupportedTupleIndex { number: ast::Number },
    #[error("break outside of loop")]
    BreakOutsideOfLoop,
}
