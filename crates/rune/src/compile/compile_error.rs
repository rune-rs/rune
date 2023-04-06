use crate::ast;
use crate::ast::{Span, Spanned, SpannedError};
use crate::compile::{IrError, IrErrorKind, ItemBuf, Location, MetaInfo};
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
    pub fn expected_meta<S>(spanned: S, meta: MetaInfo, expected: &'static str) -> Self
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
    Custom { message: Box<str> },
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
    #[error("Failed to load `{path}`: {error}")]
    FileError {
        path: PathBuf,
        #[source]
        error: io::Error,
    },
    #[error("Error during constant evaluation: {msg}")]
    ConstError { msg: &'static str },
    #[error("Experimental feature: {msg}")]
    Experimental { msg: &'static str },
    #[error("File not found, expected a module file like `{path}.rn`")]
    ModNotFound { path: PathBuf },
    #[error("Module `{item}` has already been loaded")]
    ModAlreadyLoaded {
        item: ItemBuf,
        existing: (SourceId, Span),
    },
    #[error("Variable `{name}` conflicts")]
    VariableConflict { name: String, existing_span: Span },
    #[error("Missing macro `{item}`")]
    MissingMacro { item: ItemBuf },
    #[error("{error}")]
    CallMacroError { item: ItemBuf, error: Error },
    #[error("No local variable `{name}`")]
    MissingLocal { name: String },
    #[error("Missing item `{item}`")]
    MissingItem { item: ItemBuf },
    #[error("Unsupported crate prefix `::`")]
    UnsupportedGlobal,
    #[error("Cannot load modules using a source without an associated URL")]
    UnsupportedModuleSource,
    #[error("Cannot load modules relative to `{root}`")]
    UnsupportedModuleRoot { root: PathBuf },
    #[error("Cannot load module for `{item}`")]
    UnsupportedModuleItem { item: ItemBuf },
    #[error("Wildcard support not supported in this position")]
    UnsupportedWildcard,
    #[error("Keyword `self` not supported here")]
    UnsupportedSelf,
    #[error("Unsupported unary operator `{op}`")]
    UnsupportedUnaryOp { op: ast::UnOp },
    #[error("Unsupported binary operator `{op}`")]
    UnsupportedBinaryOp { op: ast::BinOp },
    #[error("Item `{meta}` is not an object")]
    UnsupportedLitObject { meta: MetaInfo },
    #[error("Missing field `{field}` in declaration of `{item}`")]
    LitObjectMissingField { field: Box<str>, item: ItemBuf },
    #[error("Field `{field}` is not a field in `{item}`")]
    LitObjectNotField { field: Box<str>, item: ItemBuf },
    #[error("Cannot assign to expression")]
    UnsupportedAssignExpr,
    #[error("Unsupported binary expression")]
    UnsupportedBinaryExpr,
    #[error("Cannot take reference of expression")]
    UnsupportedRef,
    #[error("Unsupported select pattern")]
    UnsupportedSelectPattern,
    #[error("Unsupported field access")]
    BadFieldAccess,
    #[error("Wrong number of arguments, expected `{expected}` but got `{actual}`")]
    UnsupportedArgumentCount {
        meta: MetaInfo,
        expected: usize,
        actual: usize,
    },
    #[error("Item `{meta}` is not supported here")]
    UnsupportedPattern { meta: MetaInfo },
    #[error("This kind of expression is not supported as a pattern")]
    UnsupportedPatternExpr,
    #[error("Not a valid binding")]
    UnsupportedBinding,
    #[error("Floating point numbers cannot be used in patterns")]
    MatchFloatInPattern,
    #[error("Duplicate key in literal object")]
    DuplicateObjectKey { existing: Span, object: Span },
    #[error("Expression `yield` must be used in function or closure")]
    YieldOutsideFunction,
    #[error("Expression `await` must be used inside an async function or closure")]
    AwaitOutsideFunction,
    #[error("Instance function declared outside of `impl` block")]
    InstanceFunctionOutsideImpl,
    #[error("Import `{item}` (imported in prelude) does not exist")]
    MissingPreludeModule { item: ItemBuf },
    #[error("Unsupported tuple index `{number}`")]
    UnsupportedTupleIndex { number: ast::Number },
    #[error("Break outside of loop")]
    BreakOutsideOfLoop,
    #[error("Continue outside of loop")]
    ContinueOutsideOfLoop,
    #[error("Multiple `default` branches in select")]
    SelectMultipleDefaults,
    #[error("Expected expression to be terminated by a semicolon `;`")]
    ExpectedBlockSemiColon { followed_span: Span },
    #[error("Macro call must be terminated by a semicolon `;`")]
    ExpectedMacroSemi,
    #[error("An `fn` can't both be `async` and `const` at the same time")]
    FnConstAsyncConflict,
    #[error("A block can't both be `async` and `const` at the same time")]
    BlockConstAsyncConflict,
    #[error("Const functions can't be generators")]
    FnConstNotGenerator,
    #[error("Unsupported closure kind")]
    ClosureKind,
    #[error("Keyword `crate` is only suppoted in the first location of a path")]
    UnsupportedCrate,
    #[error("Keyword `Self` is only supported inside of `impl` blocks")]
    UnsupportedSelfType,
    #[error("Keyword `self` cannot be used here")]
    UnsupportedSelfValue,
    #[error("Keyword `super` is not supported at the root module level")]
    UnsupportedSuper,
    #[error("Keyword `super` can't be used in paths starting with `Self`")]
    UnsupportedSuperInSelfType,
    #[error("Path component cannot follow a generic argument")]
    UnsupportedAfterGeneric,
    #[error("Another segment can't follow wildcard `*` or group imports")]
    IllegalUseSegment,
    #[error("Use aliasing is not supported for wildcard `*` or group imports")]
    UseAliasNotSupported,
    #[error("Conflicting function signature already exists `{existing}`")]
    FunctionConflict { existing: DebugSignature },
    #[error("Conflicting function hash already exists `{hash}`")]
    FunctionReExportConflict { hash: Hash },
    #[error("Conflicting constant registered for `{item}` on hash `{hash}`")]
    ConstantConflict { item: ItemBuf, hash: Hash },
    #[error("Unsupported meta type for item `{existing}`")]
    UnsupportedMeta { existing: ItemBuf },
    #[error("Missing static string for hash `{hash}` and slot `{slot}`")]
    StaticStringMissing { hash: Hash, slot: usize },
    #[error("Missing static byte string for hash `{hash}` and slot `{slot}`")]
    StaticBytesMissing { hash: Hash, slot: usize },
    #[error(
        "Conflicting static string for hash `{hash}`
        between `{existing:?}` and `{current:?}`"
    )]
    StaticStringHashConflict {
        hash: Hash,
        current: String,
        existing: String,
    },
    #[error(
        "Conflicting static string for hash `{hash}`
        between `{existing:?}` and `{current:?}`"
    )]
    StaticBytesHashConflict {
        hash: Hash,
        current: Vec<u8>,
        existing: Vec<u8>,
    },
    #[error("Missing static object keys for hash `{hash}` and slot `{slot}`")]
    StaticObjectKeysMissing { hash: Hash, slot: usize },
    #[error(
        "Conflicting static object keys for hash `{hash}`
        between `{existing:?}` and `{current:?}`"
    )]
    StaticObjectKeysHashConflict {
        hash: Hash,
        current: Box<[String]>,
        existing: Box<[String]>,
    },
    #[error("Duplicate label `{label}`")]
    DuplicateLabel { label: Label },
    #[error("Missing label `{label}`")]
    MissingLabel { label: Label },
    #[error("Missing loop label `{label}`")]
    MissingLoopLabel { label: Box<str> },
    #[error("Base offset overflow")]
    BaseOverflow,
    #[error("Offset overflow")]
    OffsetOverflow,
    #[error("Segment is only supported in the first position")]
    ExpectedLeadingPathSegment,
    #[error("Visibility modifier not supported")]
    UnsupportedVisibility,
    #[error("Expected {expected} but got `{meta}`")]
    ExpectedMeta {
        expected: &'static str,
        meta: MetaInfo,
    },
    #[error("No such built-in macro `{name}`")]
    NoSuchBuiltInMacro { name: Box<str> },
    #[error("Variable moved")]
    VariableMoved { moved_at: Span },
    #[error("Unsupported generic argument")]
    UnsupportedGenerics,
    #[error("Attribute `#[test]` is not supported on nested items")]
    NestedTest { nested_span: Span },
    #[error("Attribute `#[bench]` is not supported on nested items")]
    NestedBench { nested_span: Span },
    #[error("Missing function with hash `{hash}`")]
    MissingFunctionHash { hash: Hash },
    #[error("Conflicting function already exists `{hash}`")]
    FunctionConflictHash { hash: Hash },
    #[error("Non-exhaustive pattern for `{item}`")]
    PatternMissingFields {
        item: ItemBuf,
        fields: Box<[Box<str>]>,
    },
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
    pub item: ItemBuf,
}
