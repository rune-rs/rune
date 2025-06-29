use core::convert::Infallible;
use core::fmt;

#[cfg(feature = "std")]
use std::io;
#[cfg(feature = "std")]
use std::path::PathBuf;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, String, Vec};
use crate::ast;
use crate::ast::unescape;
use crate::ast::{Span, Spanned};
use crate::compile::ir;
use crate::compile::{HasSpan, Location, MetaInfo, Visibility};
use crate::hash::TooManyParameters;
use crate::indexing::items::{GuardMismatch, MissingLastId};
use crate::macros::{SyntheticId, SyntheticKind};
use crate::parse::{Expectation, IntoExpectation, LexerMode};
use crate::runtime::debug::DebugSignature;
use crate::runtime::unit::EncodeError;
use crate::runtime::{
    AccessError, AnyObjError, ExpectedType, RuntimeError, TypeInfo, TypeOf, VmError,
};
#[cfg(feature = "std")]
use crate::source;
use crate::{Hash, Item, ItemBuf, SourceId};

/// An error raised by the compiler.
#[derive(Debug, Spanned)]
pub struct Error {
    // The span the error is associated with.
    #[rune(span)]
    span: Span,
    // Errors are exempt from fallible allocations since they're not commonly
    // constructed.
    kind: rust_alloc::boxed::Box<ErrorKind>,
}

impl Error {
    /// Construct a new compile error.
    pub(crate) fn new<S, K>(span: S, kind: K) -> Self
    where
        S: Spanned,
        ErrorKind: From<K>,
    {
        Self {
            span: span.span(),
            kind: rust_alloc::boxed::Box::new(ErrorKind::from(kind)),
        }
    }

    /// Construct an error which is made of a single message.
    pub fn msg<S, M>(span: S, message: M) -> Self
    where
        S: Spanned,
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            span: span.span(),
            kind: rust_alloc::boxed::Box::new(ErrorKind::msg(message)),
        }
    }

    /// Get the kind of the error.
    #[cfg(feature = "emit")]
    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Convert into the kind of the error.
    #[cfg(test)]
    pub(crate) fn into_kind(self) -> ErrorKind {
        *self.kind
    }
}

impl core::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl From<Infallible> for Error {
    #[inline]
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl<S, E> From<HasSpan<S, E>> for Error
where
    S: Spanned,
    ErrorKind: From<E>,
{
    fn from(spanned: HasSpan<S, E>) -> Self {
        Self::new(spanned.span(), spanned.into_inner())
    }
}

impl From<TooManyParameters> for ErrorKind {
    #[inline]
    fn from(error: TooManyParameters) -> Self {
        ErrorKind::TooManyParameters(error)
    }
}

impl From<fmt::Error> for ErrorKind {
    #[inline]
    fn from(fmt::Error: fmt::Error) -> Self {
        ErrorKind::FormatError
    }
}

impl From<syntree::Error<alloc::Error>> for ErrorKind {
    #[inline]
    fn from(error: syntree::Error<alloc::Error>) -> Self {
        ErrorKind::Syntree(error)
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for ErrorKind {
    #[inline]
    fn from(error: io::Error) -> Self {
        Self::from(anyhow::Error::from(error))
    }
}

impl From<ir::scopes::MissingLocal> for ErrorKind {
    #[inline]
    fn from(error: ir::scopes::MissingLocal) -> Self {
        ErrorKind::MissingLocal { name: error.0 }
    }
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for ErrorKind {
    #[inline]
    fn from(error: anyhow::Error) -> Self {
        ErrorKind::Custom { error }
    }
}

impl From<&'static str> for ErrorKind {
    #[inline]
    fn from(value: &'static str) -> Self {
        ErrorKind::msg(value)
    }
}

// NB: Sometimes errors are boxed because they're so big.
impl<T> From<Box<T>> for ErrorKind
where
    ErrorKind: From<T>,
{
    #[inline]
    fn from(kind: Box<T>) -> Self {
        ErrorKind::from(Box::into_inner(kind))
    }
}

// TODO: remove implementation.
impl<T> From<rust_alloc::boxed::Box<T>> for ErrorKind
where
    ErrorKind: From<T>,
{
    #[inline]
    fn from(kind: rust_alloc::boxed::Box<T>) -> Self {
        ErrorKind::from(*kind)
    }
}

impl From<ExpectedType> for ErrorKind {
    #[inline]
    fn from(error: ExpectedType) -> Self {
        ErrorKind::ExpectedType {
            actual: error.actual,
            expected: error.expected,
        }
    }
}

impl From<alloc::Error> for rust_alloc::boxed::Box<ErrorKind> {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        rust_alloc::boxed::Box::new(ErrorKind::from(error))
    }
}

impl Error {
    /// Error when we got mismatched meta.
    pub fn expected_meta<S>(spanned: S, meta: MetaInfo, expected: &'static str) -> Self
    where
        S: Spanned,
    {
        Self::new(spanned, ErrorKind::ExpectedMeta { meta, expected })
    }

    /// Construct an resolve expected error.
    pub(crate) fn expected<A, E>(actual: A, expected: E) -> Self
    where
        A: IntoExpectation + Spanned,
        E: IntoExpectation,
    {
        Self::new(
            actual.span(),
            ErrorKind::Expected {
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
            ErrorKind::Unsupported {
                what: what.into_expectation(),
            },
        )
    }

    /// An error raised when we expect a certain constant value but get another.
    pub(crate) fn expected_type<E>(spanned: impl Spanned, actual: TypeInfo) -> Self
    where
        E: TypeOf,
    {
        Self::new(
            spanned,
            IrErrorKind::Expected {
                expected: TypeInfo::from(E::STATIC_TYPE_INFO),
                actual,
            },
        )
    }
}

/// Compiler error.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum ErrorKind {
    #[cfg(feature = "anyhow")]
    Custom {
        error: anyhow::Error,
    },
    #[cfg(not(feature = "anyhow"))]
    Custom {
        error: String,
    },
    AllocError {
        error: alloc::Error,
    },
    IrError(IrErrorKind),
    MetaError(MetaError),
    AccessError(AccessError),
    VmError(VmError),
    EncodeError(EncodeError),
    MissingLastId(MissingLastId),
    GuardMismatch(GuardMismatch),
    MissingScope(MissingScope),
    PopError(PopError),
    UnescapeError(unescape::ErrorKind),
    Syntree(syntree::Error<alloc::Error>),
    TooManyParameters(TooManyParameters),
    FormatError,
    #[cfg(feature = "std")]
    SourceError {
        path: PathBuf,
        error: source::FromPathError,
    },
    ExpectedType {
        actual: TypeInfo,
        expected: TypeInfo,
    },
    Expected {
        actual: Expectation,
        expected: Expectation,
    },
    Unsupported {
        what: Expectation,
    },
    #[cfg(feature = "std")]
    ModNotFound {
        path: PathBuf,
    },
    ModAlreadyLoaded {
        item: ItemBuf,
        #[cfg(feature = "emit")]
        existing: (SourceId, Span),
    },
    MissingMacro {
        item: ItemBuf,
    },
    MissingSelf,
    MissingLocal {
        name: Box<str>,
    },
    MissingItem {
        item: ItemBuf,
    },
    MissingItemHash {
        hash: Hash,
    },
    MissingItemParameters {
        item: ItemBuf,
        parameters: [Option<Hash>; 2],
    },
    UnsupportedGlobal,
    UnsupportedModuleSource,
    #[cfg(feature = "std")]
    UnsupportedModuleRoot {
        root: PathBuf,
    },
    #[cfg(feature = "std")]
    UnsupportedModuleItem {
        item: ItemBuf,
    },
    UnsupportedSelf,
    UnsupportedUnaryOp {
        op: ast::UnOp,
    },
    UnsupportedBinaryOp {
        op: ast::BinOp,
    },
    UnsupportedLitObject {
        meta: MetaInfo,
    },
    LitObjectMissingField {
        field: Box<str>,
        item: ItemBuf,
    },
    LitObjectNotField {
        field: Box<str>,
        item: ItemBuf,
    },
    UnsupportedAssignExpr,
    UnsupportedBinaryExpr,
    UnsupportedRef,
    BadArgumentCount {
        expected: usize,
        actual: usize,
    },
    UnsupportedPatternExpr,
    UnsupportedBinding,
    DuplicateObjectKey {
        #[cfg(feature = "emit")]
        existing: Span,
        #[cfg(feature = "emit")]
        object: Span,
    },
    InstanceFunctionOutsideImpl,
    UnsupportedTupleIndex {
        number: ast::Number,
    },
    BreakUnsupported,
    BreakUnsupportedValue,
    ContinueUnsupported,
    ContinueUnsupportedBlock,
    SelectMultipleDefaults,
    ExpectedBlockSemiColon {
        #[cfg(feature = "emit")]
        followed_span: Span,
    },
    FnConstAsyncConflict,
    BlockConstAsyncConflict,
    ClosureKind,
    UnsupportedSelfType,
    UnsupportedSuper,
    UnsupportedSuperInSelfType,
    UnsupportedAfterGeneric,
    IllegalUseSegment,
    UseAliasNotSupported,
    FunctionConflict {
        existing: DebugSignature,
    },
    FunctionReExportConflict {
        hash: Hash,
    },
    ConstantConflict {
        hash: Hash,
    },
    StaticStringMissing {
        hash: Hash,
        slot: usize,
    },
    StaticBytesMissing {
        hash: Hash,
        slot: usize,
    },
    StaticStringHashConflict {
        hash: Hash,
        current: String,
        existing: String,
    },
    StaticBytesHashConflict {
        hash: Hash,
        current: Vec<u8>,
        existing: Vec<u8>,
    },
    StaticObjectKeysMissing {
        hash: Hash,
        slot: usize,
    },
    StaticObjectKeysHashConflict {
        hash: Hash,
        current: Box<[String]>,
        existing: Box<[String]>,
    },
    ConflictingLabels {
        #[cfg_attr(not(feature = "emit"), allow(unused))]
        existing: Span,
    },
    DuplicateSelectDefault {
        #[cfg_attr(not(feature = "emit"), allow(unused))]
        existing: Span,
    },
    MissingLabel {
        label: Box<str>,
    },
    ExpectedLeadingPathSegment,
    UnsupportedVisibility,
    ExpectedMeta {
        expected: &'static str,
        meta: MetaInfo,
    },
    NoSuchBuiltInMacro {
        name: Box<str>,
    },
    VariableMoved {
        #[cfg(feature = "emit")]
        moved_at: Span,
    },
    UnsupportedGenerics,
    NestedTest {
        #[cfg(feature = "emit")]
        nested_span: Span,
    },
    NestedBench {
        #[cfg(feature = "emit")]
        nested_span: Span,
    },
    MissingFunctionHash {
        hash: Hash,
    },
    FunctionConflictHash {
        hash: Hash,
    },
    PatternMissingFields {
        item: ItemBuf,
        #[cfg(feature = "emit")]
        fields: Box<[Box<str>]>,
    },
    MissingLabelLocation {
        name: &'static str,
        index: usize,
    },
    MaxMacroRecursion {
        depth: usize,
        max: usize,
    },
    YieldInConst,
    AwaitInConst,
    AwaitOutsideAsync,
    ExpectedEof {
        actual: ast::Kind,
    },
    UnexpectedEof,
    BadLexerMode {
        actual: LexerMode,
        expected: LexerMode,
    },
    ExpectedEscape,
    UnterminatedStrLit,
    UnterminatedByteStrLit,
    UnterminatedCharLit,
    UnterminatedByteLit,
    ExpectedCharClose,
    ExpectedCharOrLabel,
    ExpectedByteClose,
    UnexpectedChar {
        c: char,
    },
    PrecedenceGroupRequired,
    BadSignedOutOfBounds {
        size: ast::NumberSize,
    },
    BadUnsignedOutOfBounds {
        size: ast::NumberSize,
    },
    BadFieldAccess,
    ExpectedMacroCloseDelimiter {
        expected: ast::Kind,
        actual: ast::Kind,
    },
    MultipleMatchingAttributes {
        name: &'static str,
    },
    MissingSourceId {
        source_id: SourceId,
    },
    ExpectedMultilineCommentTerm,
    BadSlice,
    BadSyntheticId {
        kind: SyntheticKind,
        id: SyntheticId,
    },
    BadCharLiteral,
    BadByteLiteral,
    BadNumberLiteral,
    AmbiguousItem {
        item: ItemBuf,
        #[cfg(feature = "emit")]
        locations: Vec<(Location, ItemBuf)>,
    },
    AmbiguousContextItem {
        item: ItemBuf,
        #[cfg(feature = "emit")]
        infos: Box<[MetaInfo]>,
    },
    NotVisible {
        #[cfg(feature = "emit")]
        chain: Vec<Location>,
        #[cfg(feature = "emit")]
        location: Location,
        visibility: Visibility,
        item: ItemBuf,
        from: ItemBuf,
    },
    NotVisibleMod {
        #[cfg(feature = "emit")]
        chain: Vec<Location>,
        #[cfg(feature = "emit")]
        location: Location,
        visibility: Visibility,
        item: ItemBuf,
        from: ItemBuf,
    },
    MissingMod {
        item: ItemBuf,
    },
    ImportCycle {
        #[cfg(feature = "emit")]
        path: Vec<ImportStep>,
    },
    ImportRecursionLimit {
        count: usize,
        #[allow(unused)]
        path: Vec<ImportStep>,
    },
    LastUseComponent,
    RttiConflict {
        hash: Hash,
    },
    TypeRttiConflict {
        hash: Hash,
    },
    ArenaWriteSliceOutOfBounds {
        index: usize,
    },
    ArenaAllocError {
        requested: usize,
    },
    UnsupportedPatternRest,
    UnsupportedMut,
    UnsupportedSuffix,
    ClosureInConst,
    AsyncBlockInConst,
    #[cfg(feature = "fmt")]
    BadSpan {
        len: usize,
    },
    UnexpectedEndOfSyntax {
        inside: Expectation,
    },
    UnexpectedEndOfSyntaxWith {
        inside: Expectation,
        expected: Expectation,
    },
    ExpectedSyntaxEnd {
        inside: Expectation,
        actual: Expectation,
    },
    #[cfg(feature = "fmt")]
    BadIndent {
        level: isize,
        indent: usize,
    },
    ExpectedSyntax {
        expected: Expectation,
        actual: Expectation,
    },
    ExpectedSyntaxIn {
        inside: Expectation,
        expected: Expectation,
        actual: Expectation,
    },
    ExpectedOne {
        inside: Expectation,
        expected: Expectation,
    },
    ExpectedAtMostOne {
        inside: Expectation,
        expected: Expectation,
        count: usize,
    },
    ExpectedAtLeastOne {
        inside: Expectation,
        expected: Expectation,
    },
    #[cfg(feature = "fmt")]
    UnsupportedDelimiter {
        expectation: Expectation,
    },
    MissingFunction {
        hash: Hash,
    },
}

impl ErrorKind {
    #[inline]
    #[cfg(feature = "anyhow")]
    pub(crate) fn msg<M>(message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self::Custom {
            error: anyhow::Error::msg(message),
        }
    }

    #[inline]
    #[cfg(not(feature = "anyhow"))]
    pub(crate) fn msg<M>(message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match crate::alloc::fmt::try_format(format_args!("{message}")) {
            Ok(string) => Self::Custom { error: string },
            Err(error) => Self::AllocError { error },
        }
    }
}

impl core::error::Error for ErrorKind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            #[cfg(feature = "anyhow")]
            ErrorKind::Custom { error } => Some(error.as_ref()),
            ErrorKind::IrError(source) => Some(source),
            ErrorKind::MetaError(source) => Some(source),
            ErrorKind::AccessError(source) => Some(source),
            ErrorKind::VmError(source) => Some(source),
            ErrorKind::EncodeError(source) => Some(source),
            ErrorKind::MissingLastId(source) => Some(source),
            ErrorKind::GuardMismatch(source) => Some(source),
            ErrorKind::MissingScope(source) => Some(source),
            ErrorKind::PopError(source) => Some(source),
            ErrorKind::UnescapeError(source) => Some(source),
            #[cfg(feature = "std")]
            ErrorKind::SourceError { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::Custom { error } => {
                error.fmt(f)?;
            }
            ErrorKind::AllocError { error } => {
                error.fmt(f)?;
            }
            ErrorKind::IrError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::MetaError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::AccessError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::VmError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::EncodeError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::MissingLastId(error) => {
                error.fmt(f)?;
            }
            ErrorKind::GuardMismatch(error) => {
                error.fmt(f)?;
            }
            ErrorKind::MissingScope(error) => {
                error.fmt(f)?;
            }
            ErrorKind::PopError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::UnescapeError(error) => {
                error.fmt(f)?;
            }
            ErrorKind::Syntree(error) => {
                error.fmt(f)?;
            }
            ErrorKind::TooManyParameters(error) => {
                error.fmt(f)?;
            }
            ErrorKind::FormatError => {
                write!(f, "Formatting error")?;
            }
            #[cfg(feature = "std")]
            ErrorKind::SourceError { path, error } => {
                write!(
                    f,
                    "Failed to load source at `{path}`: {error}",
                    path = path.display(),
                )?;
            }
            ErrorKind::ExpectedType { actual, expected } => {
                write!(f, "Expected type `{expected}` but found `{actual}`")?;
            }
            ErrorKind::Expected { actual, expected } => {
                write!(f, "Expected {expected} but got {actual}")?;
            }
            ErrorKind::Unsupported { what } => {
                write!(f, "Unsupported {what}")?;
            }
            #[cfg(feature = "std")]
            ErrorKind::ModNotFound { path } => {
                write!(
                    f,
                    "File not found, expected a module file like `{path}.rn`",
                    path = path.display()
                )?;
            }
            ErrorKind::ModAlreadyLoaded { item, .. } => {
                write!(f, "Module `{item}` has already been loaded")?;
            }
            ErrorKind::MissingMacro { item } => {
                write!(f, "Missing macro {item}")?;
            }
            ErrorKind::MissingSelf => write!(f, "No `self` in current context")?,
            ErrorKind::MissingLocal { name } => {
                write!(f, "No local variable `{name}`")?;
            }
            ErrorKind::MissingItem { item } => {
                write!(f, "Missing item {item}")?;
            }
            ErrorKind::MissingItemHash { hash } => {
                write!(
                    f,
                    "Tried to insert meta with hash `{hash}` which does not have an item",
                )?;
            }
            ErrorKind::MissingItemParameters { item, parameters } => {
                write!(f, "Missing item {}", ParameterizedItem(item, parameters))?;
            }
            ErrorKind::UnsupportedGlobal => {
                write!(f, "Unsupported crate prefix `::`")?;
            }
            ErrorKind::UnsupportedModuleSource => {
                write!(
                    f,
                    "Cannot load modules using a source without an associated URL"
                )?;
            }
            #[cfg(feature = "std")]
            ErrorKind::UnsupportedModuleRoot { root } => {
                write!(
                    f,
                    "Cannot load modules relative to `{root}`",
                    root = root.display()
                )?;
            }
            #[cfg(feature = "std")]
            ErrorKind::UnsupportedModuleItem { item } => {
                write!(f, "Cannot load module for `{item}`")?;
            }
            ErrorKind::UnsupportedSelf => {
                write!(f, "Keyword `self` not supported here")?;
            }
            ErrorKind::UnsupportedUnaryOp { op } => {
                write!(f, "Unsupported unary operator `{op}`")?;
            }
            ErrorKind::UnsupportedBinaryOp { op } => {
                write!(f, "Unsupported binary operator `{op}`")?;
            }
            ErrorKind::UnsupportedLitObject { meta } => {
                write!(f, "Item `{meta}` is not an object")?;
            }
            ErrorKind::LitObjectMissingField { field, item } => {
                write!(f, "Missing field `{field}` in declaration of `{item}`")?;
            }
            ErrorKind::LitObjectNotField { field, item } => {
                write!(f, "Field `{field}` is not a field in `{item}`")?;
            }
            ErrorKind::UnsupportedAssignExpr => {
                write!(f, "Cannot assign to expression")?;
            }
            ErrorKind::UnsupportedBinaryExpr => {
                write!(f, "Unsupported binary expression")?;
            }
            ErrorKind::UnsupportedRef => {
                write!(f, "Cannot take reference of expression")?;
            }
            ErrorKind::BadArgumentCount { expected, actual } => {
                write!(f, "Wrong number of arguments {actual}, expected {expected}",)?;
            }
            ErrorKind::UnsupportedPatternExpr => {
                write!(f, "This kind of expression is not supported as a pattern")?;
            }
            ErrorKind::UnsupportedBinding => {
                write!(f, "Not a valid binding")?;
            }
            ErrorKind::DuplicateObjectKey { .. } => {
                write!(f, "Duplicate key in literal object")?;
            }
            ErrorKind::InstanceFunctionOutsideImpl => {
                write!(f, "Instance function declared outside of `impl` block")?;
            }
            ErrorKind::UnsupportedTupleIndex { number } => {
                write!(f, "Unsupported tuple index `{number}`")?;
            }
            ErrorKind::BreakUnsupported => {
                write!(f, "Break outside of loop")?;
            }
            ErrorKind::BreakUnsupportedValue => {
                write!(
                    f,
                    "Can only break with a value inside `loop` or breakable block"
                )?;
            }
            ErrorKind::ContinueUnsupported => {
                write!(f, "Continue outside of loop")?;
            }
            ErrorKind::ContinueUnsupportedBlock => {
                write!(f, "Labeled blocks cannot be `continue`'d")?;
            }
            ErrorKind::SelectMultipleDefaults => {
                write!(f, "Multiple `default` branches in select")?;
            }
            ErrorKind::ExpectedBlockSemiColon { .. } => {
                write!(f, "Expected expression to be terminated by a semicolon `;`")?;
            }
            ErrorKind::FnConstAsyncConflict => {
                write!(
                    f,
                    "An `fn` can't both be `async` and `const` at the same time"
                )?;
            }
            ErrorKind::BlockConstAsyncConflict => {
                write!(
                    f,
                    "A block can't both be `async` and `const` at the same time"
                )?;
            }
            ErrorKind::ClosureKind => {
                write!(f, "Unsupported closure kind")?;
            }
            ErrorKind::UnsupportedSelfType => {
                write!(
                    f,
                    "Keyword `Self` is only supported inside of `impl` blocks"
                )?;
            }
            ErrorKind::UnsupportedSuper => {
                write!(
                    f,
                    "Keyword `super` is not supported at the root module level"
                )?;
            }
            ErrorKind::UnsupportedSuperInSelfType => {
                write!(
                    f,
                    "Keyword `super` can't be used in paths starting with `Self`"
                )?;
            }
            ErrorKind::UnsupportedAfterGeneric => {
                write!(
                    f,
                    "This kind of path component cannot follow a generic argument"
                )?;
            }
            ErrorKind::IllegalUseSegment => {
                write!(
                    f,
                    "Another segment can't follow wildcard `*` or group imports"
                )?;
            }
            ErrorKind::UseAliasNotSupported => {
                write!(
                    f,
                    "Use aliasing is not supported for wildcard `*` or group imports"
                )?;
            }
            ErrorKind::FunctionConflict { existing } => {
                write!(
                    f,
                    "Conflicting function signature already exists `{existing}`",
                )?;
            }
            ErrorKind::FunctionReExportConflict { hash } => {
                write!(f, "Conflicting function hash already exists `{hash}`")?;
            }
            ErrorKind::ConstantConflict { hash } => {
                write!(f, "Conflicting constant for hash `{hash}`")?;
            }
            ErrorKind::StaticStringMissing { hash, slot } => {
                write!(
                    f,
                    "Missing static string for hash `{hash}` and slot `{slot}`",
                )?;
            }
            ErrorKind::StaticBytesMissing { hash, slot } => {
                write!(
                    f,
                    "Missing static byte string for hash `{hash}` and slot `{slot}`",
                )?;
            }
            ErrorKind::StaticStringHashConflict {
                hash,
                current,
                existing,
            } => {
                write!(f,"Conflicting static string for hash `{hash}` between `{existing:?}` and `{current:?}`")?;
            }
            ErrorKind::StaticBytesHashConflict {
                hash,
                current,
                existing,
            } => {
                write!(f,"Conflicting static string for hash `{hash}` between `{existing:?}` and `{current:?}`")?;
            }
            ErrorKind::StaticObjectKeysMissing { hash, slot } => {
                write!(
                    f,
                    "Missing static object keys for hash `{hash}` and slot `{slot}`",
                    hash = hash,
                    slot = slot
                )?;
            }
            ErrorKind::StaticObjectKeysHashConflict {
                hash,
                current,
                existing,
            } => {
                write!(f,"Conflicting static object keys for hash `{hash}` between `{existing:?}` and `{current:?}`")?;
            }
            ErrorKind::ConflictingLabels { .. } => {
                write!(f, "Multiple labels provided")?;
            }
            ErrorKind::DuplicateSelectDefault { .. } => {
                write!(f, "Multiple default select branches")?;
            }
            ErrorKind::MissingLabel { label } => {
                write!(f, "Missing label '{label}")?;
            }
            ErrorKind::ExpectedLeadingPathSegment => {
                write!(f, "Segment is only supported in the first position")?;
            }
            ErrorKind::UnsupportedVisibility => {
                write!(f, "Visibility modifier not supported")?;
            }
            ErrorKind::ExpectedMeta { expected, meta } => {
                write!(f, "Expected {expected} but got `{meta}`")?;
            }
            ErrorKind::NoSuchBuiltInMacro { name } => {
                write!(f, "No such built-in macro `{name}`")?;
            }
            ErrorKind::VariableMoved { .. } => {
                write!(f, "Variable moved")?;
            }
            ErrorKind::UnsupportedGenerics => {
                write!(f, "Unsupported generic argument")?;
            }
            ErrorKind::NestedTest { .. } => {
                write!(f, "Attribute `#[test]` is not supported on nested items")?;
            }
            ErrorKind::NestedBench { .. } => {
                write!(f, "Attribute `#[bench]` is not supported on nested items")?;
            }
            ErrorKind::MissingFunctionHash { hash } => {
                write!(f, "Missing function with hash `{hash}`")?;
            }
            ErrorKind::FunctionConflictHash { hash } => {
                write!(f, "Conflicting function already exists `{hash}`")?;
            }
            ErrorKind::PatternMissingFields { item, .. } => {
                write!(f, "Non-exhaustive pattern for `{item}`")?;
            }
            ErrorKind::MissingLabelLocation { name, index } => {
                write!(
                    f,
                    "Use of label `{name}_{index}` which has no code location",
                )?;
            }
            ErrorKind::MaxMacroRecursion { depth, max } => {
                write!(
                    f,
                    "Reached macro recursion limit at {depth}, limit is {max}",
                )?;
            }
            ErrorKind::YieldInConst => {
                write!(f, "Expression `yield` inside of constant function")?;
            }
            ErrorKind::AwaitInConst => {
                write!(f, "Expression `.await` inside of constant context")?;
            }
            ErrorKind::AwaitOutsideAsync => {
                write!(f, "Expression `.await` outside of async function or block")?;
            }
            ErrorKind::ExpectedEof { actual } => {
                write!(f, "Expected end of file, but got {actual}")?;
            }
            ErrorKind::UnexpectedEof => {
                write!(f, "Unexpected end of file")?;
            }
            ErrorKind::BadLexerMode { actual, expected } => {
                write!(f, "Bad lexer mode `{actual}`, expected `{expected}`")?;
            }
            ErrorKind::ExpectedEscape => {
                write!(f, "Expected escape sequence")?;
            }
            ErrorKind::UnterminatedStrLit => {
                write!(f, "Unterminated string literal")?;
            }
            ErrorKind::UnterminatedByteStrLit => {
                write!(f, "Unterminated byte string literal")?;
            }
            ErrorKind::UnterminatedCharLit => {
                write!(f, "Unterminated character literal")?;
            }
            ErrorKind::UnterminatedByteLit => {
                write!(f, "Unterminated byte literal")?;
            }
            ErrorKind::ExpectedCharClose => {
                write!(f, "Expected character literal to be closed")?;
            }
            ErrorKind::ExpectedCharOrLabel => {
                write!(f, "Expected label or character")?;
            }
            ErrorKind::ExpectedByteClose => {
                write!(f, "Expected byte literal to be closed")?;
            }
            ErrorKind::UnexpectedChar { c } => {
                write!(f, "Unexpected character `{c}`", c = c)?;
            }
            ErrorKind::PrecedenceGroupRequired => {
                write!(f, "Group required in expression to determine precedence")?;
            }
            ErrorKind::BadSignedOutOfBounds { size } => {
                write!(
                    f,
                    "Number literal out of bounds `{}` to `{}`",
                    size.signed_min(),
                    size.signed_max(),
                )?;
            }
            ErrorKind::BadUnsignedOutOfBounds { size } => {
                write!(
                    f,
                    "Number literal out of bounds `{}` to `{}`",
                    size.unsigned_min(),
                    size.unsigned_max(),
                )?;
            }
            ErrorKind::BadFieldAccess => {
                write!(f, "Unsupported field access")?;
            }
            ErrorKind::ExpectedMacroCloseDelimiter { expected, actual } => {
                write!(f, "Expected close delimiter {expected}, but got {actual}")?;
            }
            ErrorKind::MultipleMatchingAttributes { name } => {
                write!(f, "Can only specify one attribute named `{name}`")?;
            }
            ErrorKind::MissingSourceId { source_id } => {
                write!(f, "Missing source id `{source_id}`")?;
            }
            ErrorKind::ExpectedMultilineCommentTerm => {
                write!(f, "Expected multiline comment to be terminated with a `*/`")?;
            }
            ErrorKind::BadSlice => {
                write!(f, "Tried to read bad slice from source")?;
            }
            ErrorKind::BadSyntheticId { kind, id } => {
                write!(
                    f,
                    "Tried to get bad synthetic identifier `{id}` for `{kind}`",
                )?;
            }
            ErrorKind::BadCharLiteral => {
                write!(f, "Bad character literal")?;
            }
            ErrorKind::BadByteLiteral => {
                write!(f, "Bad byte literal")?;
            }
            ErrorKind::BadNumberLiteral => {
                write!(f, "Number literal not valid")?;
            }
            ErrorKind::AmbiguousItem { item, .. } => {
                write!(f, "Item `{item}` can refer to multiple things")?;
            }
            ErrorKind::AmbiguousContextItem { item, .. } => {
                write!(
                    f,
                    "Item `{item}` can refer to multiple things from the context"
                )?;
            }
            ErrorKind::NotVisible {
                visibility,
                item,
                from,
                ..
            } => {
                write!(f,"Item `{item}` with visibility `{visibility}`, is not accessible from module `{from}`")?;
            }
            ErrorKind::NotVisibleMod {
                visibility,
                item,
                from,
                ..
            } => {
                write!(f,"Module `{item}` with {visibility} visibility, is not accessible from module `{from}`")?;
            }
            ErrorKind::MissingMod { item } => {
                write!(f, "Missing query meta for module {item}")?;
            }
            ErrorKind::ImportCycle { .. } => {
                write!(f, "Cycle in import")?;
            }
            ErrorKind::ImportRecursionLimit { count, .. } => {
                write!(f, "Import recursion limit reached ({count})", count = count)?;
            }
            ErrorKind::LastUseComponent => {
                write!(f, "Missing last use component")?;
            }
            ErrorKind::RttiConflict { hash } => {
                write!(f,"Tried to insert variant runtime type information, but conflicted with hash `{hash}`")?;
            }
            ErrorKind::TypeRttiConflict { hash } => {
                write!(
                    f,
                    "Tried to insert runtime type information, but conflicted with hash `{hash}`",
                    hash = hash
                )?;
            }
            ErrorKind::ArenaWriteSliceOutOfBounds { index } => {
                write!(
                    f,
                    "Writing arena slice out of bounds for index {index}",
                    index = index
                )?;
            }
            ErrorKind::ArenaAllocError { requested } => {
                write!(f, "Allocation error for {requested} bytes")?;
            }
            ErrorKind::UnsupportedPatternRest => {
                write!(f, "Pattern `..` is not supported in this location")?;
            }
            ErrorKind::UnsupportedMut => {
                write!(
                    f,
                    "The `mut` modifier is not supported in Rune, everything is mutable by default"
                )?;
            }
            ErrorKind::UnsupportedSuffix => {
                write!(
                    f,
                    "Unsupported suffix, expected one of `u8`, `i64`, `u64`, or `f64`"
                )?;
            }
            ErrorKind::ClosureInConst => {
                write!(f, "Closures are not supported in constant contexts")?;
            }
            ErrorKind::AsyncBlockInConst => {
                write!(f, "Async blocks are not supported in constant contexts")?;
            }
            #[cfg(feature = "fmt")]
            ErrorKind::BadSpan { len } => {
                write!(f, "Span is outside of source 0-{len}")?;
            }
            ErrorKind::UnexpectedEndOfSyntax { inside } => {
                write!(f, "Unexpected end of syntax while parsing {inside}")?;
            }
            ErrorKind::UnexpectedEndOfSyntaxWith { inside, expected } => {
                write!(
                    f,
                    "Expected {expected} but got end of syntax while parsing {inside}"
                )?;
            }
            ErrorKind::ExpectedSyntaxEnd { inside, actual } => {
                write!(
                    f,
                    "Expected end of syntax but got {actual} while parsing {inside}"
                )?;
            }
            #[cfg(feature = "fmt")]
            ErrorKind::BadIndent { level, indent } => {
                write!(f, "Got bad indent {level} with existing {indent}")?;
            }
            ErrorKind::ExpectedSyntax { expected, actual } => {
                write!(f, "Expected {expected} but got {actual}")?;
            }
            ErrorKind::ExpectedSyntaxIn {
                inside,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Expected {expected} but got {actual} while parsing {inside}"
                )?;
            }
            ErrorKind::ExpectedOne { inside, expected } => {
                write!(f, "Expected {expected} while parsing {inside}")?;
            }
            ErrorKind::ExpectedAtMostOne {
                inside,
                expected,
                count,
            } => {
                write!(
                    f,
                    "Expected one {expected} but got {count} of them while parsing {inside}"
                )?;
            }
            ErrorKind::ExpectedAtLeastOne { inside, expected } => {
                write!(f, "Expected one {expected} while parsing {inside}")?;
            }
            #[cfg(feature = "fmt")]
            ErrorKind::UnsupportedDelimiter { expectation } => {
                write!(f, "Unsupported delimiter {expectation}")?;
            }
            ErrorKind::MissingFunction { hash, .. } => {
                write!(f, "Missing function with hash {hash}")?;
            }
        }

        Ok(())
    }
}

struct ParameterizedItem<'a>(&'a Item, &'a [Option<Hash>; 2]);

impl fmt::Display for ParameterizedItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.0.iter();

        let (Some(item), Some(ty)) = (it.next_back(), it.next_back()) else {
            return self.0.fmt(f);
        };

        let mut first = false;

        for c in it {
            if first {
                write!(f, "::{c}")?;
            } else {
                write!(f, "{c}")?;
            }

            first = true;
        }

        let [ty_param, item_param] = self.1;

        if let Some(ty_param) = ty_param {
            write!(f, "::{ty}<{ty_param}>")?;
        } else {
            write!(f, "::{ty}")?;
        }

        if let Some(item_param) = item_param {
            write!(f, "::{item}<{item_param}>")?;
        } else {
            write!(f, "::{item}")?;
        }

        Ok(())
    }
}

impl From<alloc::Error> for Error {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Error::new(Span::empty(), ErrorKind::AllocError { error })
    }
}

impl From<alloc::Error> for ErrorKind {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        ErrorKind::AllocError { error }
    }
}

impl From<alloc::alloc::AllocError> for Error {
    #[inline]
    fn from(error: alloc::alloc::AllocError) -> Self {
        Self::from(alloc::Error::from(error))
    }
}

impl From<alloc::alloc::AllocError> for ErrorKind {
    #[inline]
    fn from(error: alloc::alloc::AllocError) -> Self {
        Self::from(alloc::Error::from(error))
    }
}

impl From<IrErrorKind> for ErrorKind {
    #[inline]
    fn from(error: IrErrorKind) -> Self {
        ErrorKind::IrError(error)
    }
}

impl From<MetaError> for ErrorKind {
    #[inline]
    fn from(error: MetaError) -> Self {
        ErrorKind::MetaError(error)
    }
}

impl From<AccessError> for ErrorKind {
    #[inline]
    fn from(error: AccessError) -> Self {
        ErrorKind::AccessError(error)
    }
}

impl From<VmError> for ErrorKind {
    #[inline]
    fn from(error: VmError) -> Self {
        ErrorKind::VmError(error)
    }
}

impl From<RuntimeError> for ErrorKind {
    #[inline]
    fn from(error: RuntimeError) -> Self {
        ErrorKind::VmError(VmError::new(error.into_vm_error_kind()))
    }
}

impl From<AnyObjError> for ErrorKind {
    #[inline]
    fn from(error: AnyObjError) -> Self {
        Self::from(RuntimeError::from(error))
    }
}

impl From<EncodeError> for ErrorKind {
    #[inline]
    fn from(error: EncodeError) -> Self {
        ErrorKind::EncodeError(error)
    }
}

impl From<MissingLastId> for ErrorKind {
    #[inline]
    fn from(error: MissingLastId) -> Self {
        ErrorKind::MissingLastId(error)
    }
}

impl From<GuardMismatch> for ErrorKind {
    #[inline]
    fn from(error: GuardMismatch) -> Self {
        ErrorKind::GuardMismatch(error)
    }
}

impl From<MissingScope> for ErrorKind {
    #[inline]
    fn from(error: MissingScope) -> Self {
        ErrorKind::MissingScope(error)
    }
}

impl From<PopError> for ErrorKind {
    #[inline]
    fn from(error: PopError) -> Self {
        ErrorKind::PopError(error)
    }
}

impl From<unescape::ErrorKind> for ErrorKind {
    #[inline]
    fn from(source: unescape::ErrorKind) -> Self {
        ErrorKind::UnescapeError(source)
    }
}

/// Error when encoding AST.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum IrErrorKind {
    /// Encountered an expression that is not supported as a constant
    /// expression.
    NotConst,
    /// Trying to process a cycle of constants.
    ConstCycle,
    /// Encountered a compile meta used in an inappropriate position.
    UnsupportedMeta {
        /// Unsupported compile meta.
        meta: MetaInfo,
    },
    /// A constant evaluation errored.
    Expected {
        /// The expected value.
        expected: TypeInfo,
        /// The value we got instead.
        actual: TypeInfo,
    },
    /// Exceeded evaluation budget.
    BudgetExceeded,
    /// Missing a tuple index.
    MissingIndex {
        /// The index that was missing.
        index: usize,
    },
    /// Missing an object field.
    MissingField {
        /// The field that was missing.
        field: Box<str>,
    },
    /// Error raised when trying to use a break outside of a loop.
    BreakOutsideOfLoop,
    ArgumentCountMismatch {
        actual: usize,
        expected: usize,
    },
}

impl core::error::Error for IrErrorKind {}

impl fmt::Display for IrErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IrErrorKind::NotConst => {
                write!(f, "Expected a constant expression")?;
            }
            IrErrorKind::ConstCycle => {
                write!(f, "Constant cycle detected")?;
            }
            IrErrorKind::UnsupportedMeta { meta } => {
                write!(f, "Item `{meta}` is not supported here",)?
            }
            IrErrorKind::Expected { expected, actual } => {
                write!(f, "Expected a value of type {expected} but got {actual}",)?
            }
            IrErrorKind::BudgetExceeded => {
                write!(f, "Evaluation budget exceeded")?;
            }
            IrErrorKind::MissingIndex { index } => {
                write!(f, "Missing index {index}")?;
            }
            IrErrorKind::MissingField { field } => {
                write!(f, "Missing field `{field}`")?;
            }
            IrErrorKind::BreakOutsideOfLoop => {
                write!(f, "Break outside of supported loop")?;
            }
            IrErrorKind::ArgumentCountMismatch { actual, expected } => {
                write!(
                    f,
                    "Argument count mismatch, got {actual} but expected {expected}",
                )?;
            }
        }

        Ok(())
    }
}

/// A single step in an import.
///
/// This is used to indicate a step in an import chain in an error message.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct ImportStep {
    /// The location of the import.
    pub location: Location,
    /// The item being imported.
    pub item: ItemBuf,
}

/// A meta error.
#[derive(Debug)]
pub struct MetaError {
    kind: rust_alloc::boxed::Box<MetaErrorKind>,
}

impl MetaError {
    /// Construct a new meta error.
    pub(crate) fn new<E>(kind: E) -> Self
    where
        MetaErrorKind: From<E>,
    {
        Self {
            kind: rust_alloc::boxed::Box::new(kind.into()),
        }
    }
}

impl From<alloc::Error> for MetaError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Self::new(MetaErrorKind::AllocError { error })
    }
}

impl From<alloc::alloc::AllocError> for MetaError {
    #[inline]
    fn from(error: alloc::alloc::AllocError) -> Self {
        Self::from(alloc::Error::from(error))
    }
}

#[derive(Debug)]
/// Tried to add an item that already exists.
pub(crate) enum MetaErrorKind {
    AllocError {
        error: alloc::Error,
    },
    MetaConflict {
        /// The meta we tried to insert.
        current: MetaInfo,
        /// The existing item.
        existing: MetaInfo,
        /// Parameters hash.
        parameters: Hash,
    },
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            MetaErrorKind::AllocError { error } => error.fmt(f),
            MetaErrorKind::MetaConflict {
                current,
                existing,
                parameters,
            } => {
                write!(f, "Can't insert item `{current}` ({parameters}) because conflicting meta `{existing}` already exists")
            }
        }
    }
}

impl core::error::Error for MetaError {}

#[derive(Debug)]
pub(crate) struct MissingScope(pub(crate) usize);

impl fmt::Display for MissingScope {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing scope with id {}", self.0)
    }
}

impl core::error::Error for MissingScope {}

#[derive(Debug)]
pub(crate) enum PopError {
    MissingScope(usize),
    MissingParentScope(usize),
}

impl fmt::Display for PopError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::MissingScope(id) => write!(f, "Missing scope with id {id}"),
            PopError::MissingParentScope(id) => write!(f, "Missing parent scope with id {id}"),
        }
    }
}

impl core::error::Error for PopError {}
