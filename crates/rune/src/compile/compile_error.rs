use crate::no_std as std;
use crate::no_std::io;
use crate::no_std::path::PathBuf;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::ast;
use crate::ast::{Span, Spanned, SpannedError};
use crate::compile::{IrError, IrErrorKind, ItemBuf, Location, MetaInfo, Visibility};
use crate::hir::{HirError, HirErrorKind};
use crate::macros::{SyntheticId, SyntheticKind};
use crate::parse::{Expectation, Id, IntoExpectation, ParseError, ParseErrorKind};
use crate::runtime::debug::DebugSignature;
use crate::runtime::Label;
use crate::{Error, Hash, SourceId};

error! {
    /// An error raised by the compiler.
    #[derive(Debug)]
    pub struct CompileError {
        kind: CompileErrorKind,
    }

    impl From<ParseError>;
    impl From<IrError>;
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

    /// Construct an resolve expected error.
    pub(crate) fn expected<A, E>(actual: A, expected: E) -> Self
    where
        A: IntoExpectation + Spanned,
        E: IntoExpectation,
    {
        Self::new(
            actual.span(),
            ResolveErrorKind::Expected {
                actual: actual.into_expectation(),
                expected: expected.into_expectation(),
            },
        )
    }
}

/// Compiler error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum CompileErrorKind {
    #[error("{message}")]
    Custom { message: Box<str> },
    #[error("{error}")]
    IrError {
        #[source]
        #[from]
        error: IrErrorKind,
    },
    #[error("{0}")]
    QueryError(#[from] QueryErrorKind),
    #[error("{0}")]
    ResolveError(#[from] ResolveErrorKind),
    #[error("{error}")]
    ParseError {
        #[source]
        #[from]
        error: ParseErrorKind,
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
    #[error("This kind of expression is not supported as a pattern")]
    UnsupportedPatternExpr,
    #[error("Not a valid binding")]
    UnsupportedBinding,
    #[error("Duplicate key in literal object")]
    DuplicateObjectKey { existing: Span, object: Span },
    #[error("Expression `yield` must be used in function or closure")]
    YieldOutsideFunction,
    #[error("Expression `await` must be used inside an async function or closure")]
    AwaitOutsideFunction,
    #[error("Instance function declared outside of `impl` block")]
    InstanceFunctionOutsideImpl,
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
    #[error("An `fn` can't both be `async` and `const` at the same time")]
    FnConstAsyncConflict,
    #[error("A block can't both be `async` and `const` at the same time")]
    BlockConstAsyncConflict,
    #[error("Const functions can't be generators")]
    FnConstNotGenerator,
    #[error("Unsupported closure kind")]
    ClosureKind,
    #[error("Keyword `Self` is only supported inside of `impl` blocks")]
    UnsupportedSelfType,
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

/// Error raised during queries.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum QueryErrorKind {
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

/// The kind of a resolve error.
#[derive(Debug, Clone, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum ResolveErrorKind {
    #[error("Expected `{expected}`, but got `{actual}`")]
    Expected {
        actual: Expectation,
        expected: Expectation,
    },
    #[error("Tried to read bad slice from source")]
    BadSlice,
    #[error("Tried to get bad synthetic identifier `{id}` for `{kind}`")]
    BadSyntheticId {
        kind: SyntheticKind,
        id: SyntheticId,
    },
    #[error("Bad escape sequence")]
    BadEscapeSequence,
    #[error("Bad unicode escape")]
    BadUnicodeEscape,
    #[error(
        "This form of character escape may only be used with characters in the range [\\x00-\\x7f]"
    )]
    BadHexEscapeChar,
    #[error(
        "This form of byte escape may only be used with characters in the range [\\x00-\\xff]"
    )]
    BadHexEscapeByte,
    #[error("Bad byte escape")]
    BadByteEscape,
    #[error("Bad character literal")]
    BadCharLiteral,
    #[error("Bad byte literal")]
    BadByteLiteral,
    #[error("Unicode escapes are not supported as a byte or byte string")]
    BadUnicodeEscapeInByteString,
    #[error("Number literal not valid")]
    BadNumberLiteral,
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
