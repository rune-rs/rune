use core::fmt;

use crate::no_std as std;
use crate::no_std::io;
use crate::no_std::path::PathBuf;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::hir::scopes::{MissingScope, PopError};
use crate::compile::{HasSpan, IrValue, ItemBuf, Location, MetaInfo, Visibility};
use crate::macros::{SyntheticId, SyntheticKind};
use crate::parse::{Expectation, Id, IntoExpectation, LexerMode};
use crate::runtime::debug::DebugSignature;
use crate::runtime::unit::EncodeError;
use crate::runtime::{AccessError, TypeInfo, TypeOf};
use crate::shared::scopes::MissingLocal;
use crate::shared::MissingLastId;
use crate::{Hash, SourceId};

/// An error raised by the compiler.
#[derive(Debug)]
pub struct Error {
    span: Span,
    kind: Box<CompileErrorKind>,
}

impl Error {
    /// Construct a new compile error.
    #[allow(unused)]
    pub(crate) fn new<S, K>(spanned: S, kind: K) -> Self
    where
        S: Spanned,
        CompileErrorKind: From<K>,
    {
        Self {
            span: spanned.span(),
            kind: Box::new(CompileErrorKind::from(kind)),
        }
    }

    /// Construct an error which is made of a single message.
    pub fn msg<S, M>(spanned: S, message: M) -> Self
    where
        S: Spanned,
        M: fmt::Display,
    {
        Self {
            span: Spanned::span(&spanned),
            kind: Box::new(CompileErrorKind::Custom {
                message: message.to_string().into(),
            }),
        }
    }

    /// Get the kind of the error.
    #[cfg(feature = "emit")]
    pub(crate) fn kind(&self) -> &CompileErrorKind {
        &self.kind
    }

    /// Convert into the kind of the error.
    #[cfg(test)]
    pub(crate) fn into_kind(self) -> CompileErrorKind {
        *self.kind
    }
}

impl Spanned for Error {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

impl crate::no_std::error::Error for Error {
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Display::fmt(&self.kind, f)
    }
}

impl<E> From<HasSpan<E>> for Error
where
    CompileErrorKind: From<E>,
{
    fn from(spanned: HasSpan<E>) -> Self {
        Error {
            span: spanned.span,
            kind: Box::new(CompileErrorKind::from(spanned.error)),
        }
    }
}

impl From<MissingLocal<'_>> for CompileErrorKind {
    #[inline]
    fn from(MissingLocal(name): MissingLocal<'_>) -> Self {
        CompileErrorKind::MissingLocal { name: name.into() }
    }
}

impl From<&'static str> for CompileErrorKind {
    #[inline]
    fn from(value: &'static str) -> Self {
        CompileErrorKind::Custom {
            message: Box::from(value),
        }
    }
}

// NB: Sometimes errors are boxed because they're so big.
impl<T> From<Box<T>> for CompileErrorKind
where
    CompileErrorKind: From<T>,
{
    #[inline]
    fn from(kind: Box<T>) -> Self {
        CompileErrorKind::from(*kind)
    }
}

impl Error {
    /// Construct a factor for unsupported super.
    pub fn unsupported_super<S>(spanned: S) -> impl FnOnce() -> Self
    where
        S: Spanned,
    {
        || Error::new(spanned, CompileErrorKind::UnsupportedSuper)
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
            CompileErrorKind::Expected {
                actual: actual.into_expectation(),
                expected: expected.into_expectation(),
            },
        )
    }

    /// Construct an unsupported error.
    pub(crate) fn unsupported<T, E>(actual: T, what: E) -> Self
    where
        T: Spanned,
        E: IntoExpectation,
    {
        Self::new(
            actual.span(),
            CompileErrorKind::Unsupported {
                what: what.into_expectation(),
            },
        )
    }

    /// An error raised when we expect a certain constant value but get another.
    pub(crate) fn expected_type<S, E>(spanned: S, actual: &IrValue) -> Self
    where
        S: Spanned,
        E: TypeOf,
    {
        Self::new(
            spanned,
            IrErrorKind::Expected {
                expected: E::type_info(),
                actual: actual.type_info(),
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
    #[error("Expected `{expected}`, but got `{actual}`")]
    Expected {
        actual: Expectation,
        expected: Expectation,
    },
    #[error("Unsupported `{what}`")]
    Unsupported { what: Expectation },
    #[error("{0}")]
    IrError(#[from] IrErrorKind),
    #[error("{0}")]
    QueryError(#[from] QueryErrorKind),
    #[error("{0}")]
    MetaConflict(#[from] MetaConflict),
    #[error("{0}")]
    ResolveError(#[from] ResolveErrorKind),
    #[error("{0}")]
    ParseError(#[from] ParseErrorKind),
    #[error("{0}")]
    AccessError(#[from] AccessError),
    #[error("{0}")]
    HirError(#[from] HirErrorKind),
    #[error("{0}")]
    EncodeError(#[from] EncodeError),
    #[error("{0}")]
    MissingLastId(#[from] MissingLastId),
    #[error("{0}")]
    MissingScope(#[from] MissingScope),
    #[error("{0}")]
    PopError(#[from] PopError),
    #[error("Failed to load `{path}`: {error}")]
    FileError {
        path: PathBuf,
        #[source]
        error: io::Error,
    },
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
    #[error("No local variable `{name}`")]
    MissingLocal { name: String },
    #[error("Missing item `{item}`")]
    MissingItem { item: ItemBuf },
    #[error("Missing item `{item} {parameters:?}`")]
    MissingItemParameters {
        item: ItemBuf,
        parameters: Box<[Option<Hash>]>,
    },
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
    UnsupportedArgumentCount { expected: usize, actual: usize },
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
    #[error("This kind of path component cannot follow a generic argument")]
    UnsupportedAfterGeneric,
    #[error("Another segment can't follow wildcard `*` or group imports")]
    IllegalUseSegment,
    #[error("Use aliasing is not supported for wildcard `*` or group imports")]
    UseAliasNotSupported,
    #[error("Conflicting function signature already exists `{existing}`")]
    FunctionConflict { existing: DebugSignature },
    #[error("Conflicting function hash already exists `{hash}`")]
    FunctionReExportConflict { hash: Hash },
    #[error("Conflicting constant for hash `{hash}`")]
    ConstantConflict { hash: Hash },
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
    #[error("Missing loop label `{label}`")]
    MissingLoopLabel { label: Box<str> },
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
    #[error("Use of label `{name}_{index}` which has no code location")]
    MissingLabelLocation { name: &'static str, index: usize },
    #[error("Reached macro recursion limit at {depth}, limit is {max}")]
    MaxMacroRecursion { depth: usize, max: usize },
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
    #[error("Item `{item}` can refer to multiple things from the context")]
    AmbiguousContextItem {
        item: ItemBuf,
        infos: Box<[MetaInfo]>,
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
    #[error("Tried to insert meta with hash `{hash}` which does not have an item")]
    MissingItem { hash: Hash },
    #[error("Missing query meta for module {item}")]
    MissingMod { item: ItemBuf },
    #[error("Cycle in import")]
    ImportCycle { path: Vec<ImportStep> },
    #[error("Import recursion limit reached ({count})")]
    ImportRecursionLimit { count: usize, path: Vec<ImportStep> },
    #[error("Missing last use component")]
    LastUseComponent,
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

/// Error when parsing.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum ParseErrorKind {
    #[error("Expected end of file, but got `{actual}`")]
    ExpectedEof { actual: ast::Kind },
    #[error("Unexpected end of file")]
    UnexpectedEof,
    #[error("Bad lexer mode `{actual}`, expected `{expected}`")]
    BadLexerMode {
        actual: LexerMode,
        expected: LexerMode,
    },
    #[error("Expected escape sequence")]
    ExpectedEscape,
    #[error("Unterminated string literal")]
    UnterminatedStrLit,
    #[error("Unterminated byte string literal")]
    UnterminatedByteStrLit,
    #[error("Unterminated character literal")]
    UnterminatedCharLit,
    #[error("Unterminated byte literal")]
    UnterminatedByteLit,
    #[error("Expected character literal to be closed")]
    ExpectedCharClose,
    #[error("Expected label or character")]
    ExpectedCharOrLabel,
    #[error("Expected byte literal to be closed")]
    ExpectedByteClose,
    #[error("Unexpected character `{c}`")]
    UnexpectedChar { c: char },
    #[error("Group required in expression to determine precedence")]
    PrecedenceGroupRequired,
    #[error("Number literal out of bounds `-9223372036854775808` to `9223372036854775807`")]
    BadNumberOutOfBounds,
    #[error("Unsupported field access")]
    BadFieldAccess,
    #[error("Expected close delimiter `{expected}`, but got `{actual}`")]
    ExpectedMacroCloseDelimiter {
        expected: ast::Kind,
        actual: ast::Kind,
    },
    #[error("Bad number literal")]
    BadNumber,
    #[error("Can only specify one attribute named `{name}`")]
    MultipleMatchingAttributes { name: &'static str },
    #[error("Missing source id `{source_id}`")]
    MissingSourceId { source_id: SourceId },
    #[error("Expected multiline comment to be terminated with a `*/`")]
    ExpectedMultilineCommentTerm,
}

/// Error when encoding AST.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum IrErrorKind {
    /// Encountered an expression that is not supported as a constant
    /// expression.
    #[error("Expected a constant expression")]
    NotConst,
    /// Trying to process a cycle of constants.
    #[error("Constant cycle detected")]
    ConstCycle,
    /// Encountered a compile meta used in an inappropriate position.
    #[error("Item `{meta}` is not supported here")]
    UnsupportedMeta {
        /// Unsupported compile meta.
        meta: MetaInfo,
    },
    /// A constant evaluation errored.
    #[error("Expected a value of type {expected} but got {actual}")]
    Expected {
        /// The expected value.
        expected: TypeInfo,
        /// The value we got instead.
        actual: TypeInfo,
    },
    /// Exceeded evaluation budget.
    #[error("Evaluation budget exceeded")]
    BudgetExceeded,
    /// Missing a tuple index.
    #[error("Missing index {index}")]
    MissingIndex {
        /// The index that was missing.
        index: usize,
    },
    /// Missing an object field.
    #[error("Missing field `{field}`")]
    MissingField {
        /// The field that was missing.
        field: Box<str>,
    },
    /// Missing const or local with the given name.
    #[error("No constant or local matching `{name}`")]
    MissingConst {
        /// Name of the missing thing.
        name: Box<str>,
    },
    /// Error raised when trying to use a break outside of a loop.
    #[error("Break outside of supported loop")]
    BreakOutsideOfLoop,
    #[error("Argument count mismatch, got {actual} but expected {expected}")]
    ArgumentCountMismatch { actual: usize, expected: usize },
}

/// The kind of a hir error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum HirErrorKind {
    #[error("Writing arena slice out of bounds for index {index}")]
    ArenaWriteSliceOutOfBounds { index: usize },
    #[error("Allocation error for {requested} bytes")]
    ArenaAllocError { requested: usize },
    #[error("Pattern `..` is not supported in this location")]
    UnsupportedPatternRest,
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

#[derive(Debug, Error)]
/// Tried to add an item that already exists.
#[error("Can't insert item `{current}` ({parameters}) because conflicting meta `{existing}` already exists")]
pub(crate) struct MetaConflict {
    /// The meta we tried to insert.
    pub(crate) current: MetaInfo,
    /// The existing item.
    pub(crate) existing: MetaInfo,
    /// Parameters hash.
    pub(crate) parameters: Hash,
}
