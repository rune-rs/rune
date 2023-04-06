use crate::ast::{Spanned, SpannedError};
use crate::compile::{IrValue, MetaInfo};
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
    Custom { message: Box<str> },
    /// An access error raised during compilation.
    #[error("Access error: {error}")]
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
    #[error("Scope error: {error}")]
    ScopeError {
        /// The kind of the scope error.
        #[source]
        #[from]
        error: ScopeErrorKind,
    },
    /// A HIR error.
    #[error("HIR error: {error}")]
    HirError {
        /// The kind of the scope error.
        #[source]
        #[from]
        error: HirErrorKind,
    },
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
    /// Integer underflow.
    #[error("Integer underflow")]
    IntegerUnderflow,
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
    /// Missing local with the given name.
    #[error("Missing local `{name}`")]
    MissingLocal {
        /// Name of the missing local.
        name: Box<str>,
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
    #[error("Function not found")]
    FnNotFound,
    #[error("Argument count mismatch, got {actual} but expected {expected}")]
    ArgumentCountMismatch { actual: usize, expected: usize },
    #[error("Value `{value}` is outside of the supported integer range")]
    NotInteger { value: num::BigInt },
}
