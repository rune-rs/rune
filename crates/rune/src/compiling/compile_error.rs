use crate::ast;
use crate::compiling::{ImportKey, InsertMetaError};
use crate::shared::Internal;
use crate::{
    IrError, IrErrorKind, ParseError, ParseErrorKind, QueryError, QueryErrorKind, Spanned,
};
use runestick::debug::DebugSignature;
use runestick::{CompileMeta, Hash, Item, Label, SourceId, Span};
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
}

impl CompileError {
    /// Construct an internal error.
    ///
    /// This should be used for programming invariants of the encoder which are
    /// broken for some reason.
    pub fn internal<S>(spanned: S, message: &'static str) -> Self
    where
        S: Spanned,
    {
        CompileError::new(spanned, CompileErrorKind::Internal { message })
    }

    /// Construct a factor for unsupported super.
    pub fn unsupported_super<S>(spanned: S) -> impl FnOnce() -> Self
    where
        S: Spanned,
    {
        || CompileError::new(spanned, CompileErrorKind::UnsupportedSuper)
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
                message: "paths containing `crate` or `super` are not supported",
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

impl From<Internal> for CompileError {
    fn from(error: Internal) -> Self {
        Self {
            span: error.span(),
            kind: CompileErrorKind::Internal {
                message: error.message(),
            },
        }
    }
}

/// Compiler error.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum CompileErrorKind {
    #[error("internal compiler error: {message}")]
    Internal { message: &'static str },
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
    #[error("missing item `{item}`")]
    MissingItem { item: Item },
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
    #[error("multiple `default` branches in select")]
    SelectMultipleDefaults,
    /// Expected a block semicolon which is needed for the kind of expression.
    #[error("expected expression to be terminated by a semicolon `;`")]
    ExpectedBlockSemiColon {
        /// The following expression.
        followed_span: Span,
    },
    #[error("an `fn` can't both be `async` and `const` at the same time")]
    FnConstAsyncConflict,
    #[error("const functions can't be generators")]
    FnConstNotGenerator,
    #[error("unsupported closure kind")]
    ClosureKind,
    #[error("`crate` is only suppoted in the first location of a path")]
    UnsupportedCrate,
    #[error("`Self` is only supported inside of `impl` blocks")]
    UnsupportedSelfType,
    #[error("`self` cannot be used here")]
    UnsupportedSelfValue,
    #[error("`super` is not supported at the root module level")]
    UnsupportedSuper,
    #[error("`super` can't be used in paths starting with `Self`")]
    UnsupportedSuperInSelfType,
    #[error("another segment can't follow wildcard `*` or group imports")]
    IllegalUseSegment,
    #[error("use aliasing is not supported for wildcard `*` or group imports")]
    UseAliasNotSupported,
    /// Trying to register a conflicting function.
    #[error("conflicting function signature already exists `{existing}`")]
    FunctionConflict {
        /// The signature of an already existing function.
        existing: DebugSignature,
    },
    /// Tried to register a conflicting constant.
    #[error("conflicting constant registered for `{item}` on hash `{hash}`")]
    ConstantConflict {
        /// The item that was conflicting.
        item: Item,
        /// The conflicting hash.
        hash: Hash,
    },
    /// Tried to add an unsupported meta item to a unit.
    #[error("unsupported meta type for item `{existing}`")]
    UnsupportedMeta {
        /// The item used.
        existing: Item,
    },
    /// A static string was missing for the given hash and slot.
    #[error("missing static string for hash `{hash}` and slot `{slot}`")]
    StaticStringMissing {
        /// The hash of the string.
        hash: Hash,
        /// The slot of the string.
        slot: usize,
    },
    /// A static byte string was missing for the given hash and slot.
    #[error("missing static byte string for hash `{hash}` and slot `{slot}`")]
    StaticBytesMissing {
        /// The hash of the byte string.
        hash: Hash,
        /// The slot of the byte string.
        slot: usize,
    },
    /// A static string was missing for the given hash and slot.
    #[error(
        "conflicting static string for hash `{hash}` between `{existing:?}` and `{current:?}`"
    )]
    StaticStringHashConflict {
        /// The hash of the string.
        hash: Hash,
        /// The static string that was inserted.
        current: String,
        /// The existing static string that conflicted.
        existing: String,
    },
    /// A static byte string was missing for the given hash and slot.
    #[error(
        "conflicting static string for hash `{hash}` between `{existing:?}` and `{current:?}`"
    )]
    StaticBytesHashConflict {
        /// The hash of the byte string.
        hash: Hash,
        /// The static byte string that was inserted.
        current: Vec<u8>,
        /// The existing static byte string that conflicted.
        existing: Vec<u8>,
    },
    /// A static object keys was missing for the given hash and slot.
    #[error("missing static object keys for hash `{hash}` and slot `{slot}`")]
    StaticObjectKeysMissing {
        /// The hash of the object keys.
        hash: Hash,
        /// The slot of the object keys.
        slot: usize,
    },
    /// A static object keys was missing for the given hash and slot.
    #[error(
        "conflicting static object keys for hash `{hash}` between `{existing:?}` and `{current:?}`"
    )]
    StaticObjectKeysHashConflict {
        /// The hash of the object keys.
        hash: Hash,
        /// The static object keys that was inserted.
        current: Box<[String]>,
        /// The existing static object keys that conflicted.
        existing: Box<[String]>,
    },
    /// Tried to add a duplicate label.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// The duplicate label.
        label: Label,
    },
    /// The specified label is missing.
    #[error("missing label `{label}`")]
    MissingLabel {
        /// The missing label.
        label: Label,
    },
    /// The specified loop label is missing.
    #[error("missing loop label `{label}`")]
    MissingLoopLabel {
        /// The missing label.
        label: Box<str>,
    },
    /// Overflow error.
    #[error("base offset overflow")]
    BaseOverflow,
    /// Overflow error.
    #[error("offset overflow")]
    OffsetOverflow,
    #[error("conflicting import {key}")]
    ImportConflict { key: ImportKey },
}
