use crate::ast::{Spanned, SpannedError};
use crate::compile::{IrValue, Meta};
use crate::hir::{HirError, HirErrorKind};
use crate::parse::{ResolveError, ResolveErrorKind};
use crate::query::{QueryError, QueryErrorKind};
use crate::runtime::{AccessError, TypeInfo, TypeOf};
use crate::shared::{ScopeError, ScopeErrorKind};
use thiserror::Error;

error! {
    /// An error raised during compiling.
    #[derive(Debug)]
    pub struct IrError {
        kind: IrErrorKind,
    }

    impl From<ResolveError>;
    impl From<QueryError>;
    impl From<ScopeError>;
    impl From<HirError>;
}

impl IrError {
    /// An error raised when we expect a certain constant value but get another.
    pub(crate) fn expected<S, E>(spanned: S, actual: &IrValue) -> Self
    where
        S: Spanned,
        E: TypeOf,
    {
        IrError::new(
            spanned,
            IrErrorKind::Expected {
                expected: E::type_info(),
                actual: actual.type_info(),
            },
        )
    }

    /// Construct a callback to build an access error with the given spanned.
    pub(crate) fn access<S>(spanned: S) -> impl FnOnce(AccessError) -> Self
    where
        S: Spanned,
    {
        move |error| Self::new(spanned, error)
    }
}

impl From<IrError> for SpannedError {
    fn from(error: IrError) -> Self {
        SpannedError::new(error.span, *error.kind)
    }
}

/// Error when encoding AST.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum IrErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    /// An access error raised during compilation.
    #[error("access error: {error}")]
    AccessError {
        /// The source error.
        #[source]
        #[from]
        error: AccessError,
    },
    /// An access error raised during queries.
    #[error("{error}")]
    QueryError {
        /// The source error.
        #[source]
        #[from]
        error: Box<QueryErrorKind>,
    },
    #[error("{error}")]
    ResolveError {
        #[source]
        #[from]
        error: ResolveErrorKind,
    },
    /// A scope error.
    #[error("scope error: {error}")]
    ScopeError {
        /// The kind of the scope error.
        #[source]
        #[from]
        error: ScopeErrorKind,
    },
    /// A HIR error.
    #[error("hir error: {error}")]
    HirError {
        /// The kind of the scope error.
        #[source]
        #[from]
        error: HirErrorKind,
    },
    /// Encountered an expression that is not supported as a constant
    /// expression.
    #[error("expected a constant expression")]
    NotConst,
    /// Trying to process a cycle of constants.
    #[error("constant cycle detected")]
    ConstCycle,
    /// Encountered a compile meta used in an inappropriate position.
    #[error("{meta} is not supported here")]
    UnsupportedMeta {
        /// Unsupported compile meta.
        meta: Meta,
    },
    /// A constant evaluation errored.
    #[error("expected a value of type {expected} but got {actual}")]
    Expected {
        /// The expected value.
        expected: TypeInfo,
        /// The value we got instead.
        actual: TypeInfo,
    },
    /// Exceeded evaluation budget.
    #[error("evaluation budget exceeded")]
    BudgetExceeded,
    /// Integer underflow.
    #[error("integer underflow")]
    IntegerUnderflow,
    /// Missing a tuple index.
    #[error("missing index {index}")]
    MissingIndex {
        /// The index that was missing.
        index: usize,
    },
    /// Missing an object field.
    #[error("missing field `{field}`")]
    MissingField {
        /// The field that was missing.
        field: Box<str>,
    },
    /// Missing local with the given name.
    #[error("missing local `{name}`")]
    MissingLocal {
        /// Name of the missing local.
        name: Box<str>,
    },
    /// Missing const or local with the given name.
    #[error("no constant or local matching `{name}`")]
    MissingConst {
        /// Name of the missing thing.
        name: Box<str>,
    },
    /// Error raised when trying to use a break outside of a loop.
    #[error("break outside of supported loop")]
    BreakOutsideOfLoop,
    #[error("function not found")]
    FnNotFound,
    #[error("argument count mismatch, got {actual} but expected {expected}")]
    ArgumentCountMismatch { actual: usize, expected: usize },
    #[error("value `{value}` is outside of the supported integer range")]
    NotInteger { value: num::BigInt },
}
