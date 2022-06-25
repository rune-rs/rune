use crate::ast;
use crate::ast::{Span, Spanned, SpannedError};
use crate::compile::{IrError, IrErrorKind, Item, Location, Meta};
use crate::hir::{HirError, HirErrorKind};
use crate::parse::{ParseError, ParseErrorKind, ResolveError, ResolveErrorKind};
use crate::query::{QueryError, QueryErrorKind};
use crate::runtime::debug::DebugSignature;
use crate::runtime::Label;
use crate::{Error, Hash, SourceId};
use std::io;
use std::path::PathBuf;
use thiserror::Error;

error! {
    /// An error raised by the compiler.
    #[derive(Debug)]
    pub struct CompileError {
        kind: CompileErrorKind,
    }

    impl From<ParseError>;
    impl From<IrError>;
    impl From<QueryError>;
    impl From<ResolveError>;
    impl From<HirError>;
}

impl From<CompileError> for SpannedError {
    fn from(error: CompileError) -> Self {
        SpannedError::new(error.span, *error.kind)
    }
}

impl CompileError {
    /// Construct a factor for unsupported super.
    pub fn unsupported_super<S>(spanned: S) -> impl FnOnce() -> Self
    where
        S: Spanned,
    {
        || CompileError::new(spanned, CompileErrorKind::UnsupportedSuper)
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

    /// Error when we got mismatched meta.
    pub fn expected_meta<S>(spanned: S, meta: Meta, expected: &'static str) -> Self
    where
        S: Spanned,
    {
        Self::new(spanned, CompileErrorKind::ExpectedMeta { meta, expected })
    }
}

/// Compiler error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum CompileErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("{error}")]
    IrError {
        #[source]
        #[from]
        error: IrErrorKind,
    },
    #[error("{error}")]
    QueryError {
        #[source]
        #[from]
        error: QueryErrorKind,
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
    #[error("failed to load `{path}`: {error}")]
    ModFileError {
        path: PathBuf,
        #[source]
        error: io::Error,
    },
    #[error("error during constant evaluation: {msg}")]
    ConstError { msg: &'static str },
    #[error("experimental feature: {msg}")]
    Experimental { msg: &'static str },
    #[error("file not found, expected a module file like `{path}.rn`")]
    ModNotFound { path: PathBuf },
    #[error("module `{item}` has already been loaded")]
    ModAlreadyLoaded {
        item: Item,
        existing: (SourceId, Span),
    },
    #[error("variable `{name}` conflicts")]
    VariableConflict { name: String, existing_span: Span },
    #[error("missing macro `{item}`")]
    MissingMacro { item: Item },
    #[error("{error}")]
    CallMacroError { item: Item, error: Error },
    #[error("no local variable `{name}`")]
    MissingLocal { name: String },
    #[error("missing item `{item}`")]
    MissingItem { item: Item },
    #[error("unsupported crate prefix `::`")]
    UnsupportedGlobal,
    #[error("cannot load modules using a source without an associated URL")]
    UnsupportedModuleSource,
    #[error("cannot load modules relative to `{root}`")]
    UnsupportedModuleRoot { root: PathBuf },
    #[error("cannot load module for `{item}`")]
    UnsupportedModuleItem { item: Item },
    #[error("wildcard support not supported in this position")]
    UnsupportedWildcard,
    #[error("`self` not supported here")]
    UnsupportedSelf,
    #[error("unsupported unary operator `{op}`")]
    UnsupportedUnaryOp { op: ast::UnOp },
    #[error("unsupported binary operator `{op}`")]
    UnsupportedBinaryOp { op: ast::BinOp },
    #[error("{meta} is not an object")]
    UnsupportedLitObject { meta: Meta },
    #[error("missing field `{field}` in declaration of `{item}`")]
    LitObjectMissingField { field: Box<str>, item: Item },
    #[error("`{field}` is not a field in `{item}`")]
    LitObjectNotField { field: Box<str>, item: Item },
    #[error("cannot assign to expression")]
    UnsupportedAssignExpr,
    #[error("unsupported binary expression")]
    UnsupportedBinaryExpr,
    #[error("cannot take reference of expression")]
    UnsupportedRef,
    #[error("unsupported select pattern")]
    UnsupportedSelectPattern,
    #[error("unsupported field access")]
    BadFieldAccess,
    #[error("wrong number of arguments, expected `{expected}` but got `{actual}`")]
    UnsupportedArgumentCount {
        meta: Meta,
        expected: usize,
        actual: usize,
    },
    #[error("{meta} is not supported here")]
    UnsupportedPattern { meta: Meta },
    #[error("`..` is not supported in this location")]
    UnsupportedPatternRest,
    #[error("this kind of expression is not supported as a pattern")]
    UnsupportedPatternExpr,
    #[error("not a valid binding")]
    UnsupportedBinding,
    #[error("floating point numbers cannot be used in patterns")]
    MatchFloatInPattern,
    #[error("duplicate key in literal object")]
    DuplicateObjectKey { existing: Span, object: Span },
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
    #[error("continue outside of loop")]
    ContinueOutsideOfLoop,
    #[error("multiple `default` branches in select")]
    SelectMultipleDefaults,
    #[error("expected expression to be terminated by a semicolon `;`")]
    ExpectedBlockSemiColon { followed_span: Span },
    #[error("macro call must be terminated by a semicolon `;`")]
    ExpectedMacroSemi,
    #[error("an `fn` can't both be `async` and `const` at the same time")]
    FnConstAsyncConflict,
    #[error("a block can't both be `async` and `const` at the same time")]
    BlockConstAsyncConflict,
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
    #[error("path component cannot follow a generic argument")]
    UnsupportedAfterGeneric,
    #[error("another segment can't follow wildcard `*` or group imports")]
    IllegalUseSegment,
    #[error("use aliasing is not supported for wildcard `*` or group imports")]
    UseAliasNotSupported,
    #[error("conflicting function signature already exists `{existing}`")]
    FunctionConflict { existing: DebugSignature },
    #[error("conflicting function hash already exists `{hash}`")]
    FunctionReExportConflict { hash: Hash },
    #[error("conflicting constant registered for `{item}` on hash `{hash}`")]
    ConstantConflict { item: Item, hash: Hash },
    #[error("unsupported meta type for item `{existing}`")]
    UnsupportedMeta { existing: Item },
    #[error("missing static string for hash `{hash}` and slot `{slot}`")]
    StaticStringMissing { hash: Hash, slot: usize },
    #[error("missing static byte string for hash `{hash}` and slot `{slot}`")]
    StaticBytesMissing { hash: Hash, slot: usize },
    #[error(
        "conflicting static string for hash `{hash}`
        between `{existing:?}` and `{current:?}`"
    )]
    StaticStringHashConflict {
        hash: Hash,
        current: String,
        existing: String,
    },
    #[error(
        "conflicting static string for hash `{hash}`
        between `{existing:?}` and `{current:?}`"
    )]
    StaticBytesHashConflict {
        hash: Hash,
        current: Vec<u8>,
        existing: Vec<u8>,
    },
    #[error("missing static object keys for hash `{hash}` and slot `{slot}`")]
    StaticObjectKeysMissing { hash: Hash, slot: usize },
    #[error(
        "conflicting static object keys for hash `{hash}`
        between `{existing:?}` and `{current:?}`"
    )]
    StaticObjectKeysHashConflict {
        hash: Hash,
        current: Box<[String]>,
        existing: Box<[String]>,
    },
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: Label },
    #[error("missing label `{label}`")]
    MissingLabel { label: Label },
    #[error("missing loop label `{label}`")]
    MissingLoopLabel { label: Box<str> },
    #[error("base offset overflow")]
    BaseOverflow,
    #[error("offset overflow")]
    OffsetOverflow,
    #[error("segment is only supported in the first position")]
    ExpectedLeadingPathSegment,
    #[error("visibility modifier not supported")]
    UnsupportedVisibility,
    #[error("expected {expected} but got `{meta}`")]
    ExpectedMeta { expected: &'static str, meta: Meta },
    #[error("no such built-in macro `{name}`")]
    NoSuchBuiltInMacro { name: Box<str> },
    #[error("variable moved")]
    VariableMoved { moved_at: Span },
    #[error("unsupported generic argument")]
    UnsupportedGenerics,
    #[error("#[test] attributes are not supported on nested items")]
    NestedTest { nested_span: Span },
    #[error("#[bench] attributes are not supported on nested items")]
    NestedBench { nested_span: Span },
    #[error("missing function with hash `{hash}`")]
    MissingFunctionHash { hash: Hash },
    #[error("conflicting function already exists `{hash}`")]
    FunctionConflictHash { hash: Hash },
    #[error("non-exhaustive pattern for `{item}`")]
    PatternMissingFields { item: Item, fields: Box<[Box<str>]> },
}

/// A single step in an import.
///
/// This is used to indicate a step in an import chain in an error message.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ImportStep {
    /// The location of the import.
    pub location: Location,
    /// The item being imported.
    pub item: Item,
}
