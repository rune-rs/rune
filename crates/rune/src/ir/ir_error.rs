use crate::ir::IrValue;
use crate::shared::{ScopeError, ScopeErrorKind};
use crate::{QueryError, QueryErrorKind, ResolveError, ResolveErrorKind, Spanned};
use runestick::{AccessError, CompileMeta, SpannedError, TypeInfo, TypeOf};
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
}

impl IrError {
    /// An error raised when we expect a certain constant value but get another.
    pub fn expected<S, E>(spanned: S, actual: &IrValue) -> Self
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
    pub fn access<S>(spanned: S) -> impl FnOnce(AccessError) -> Self
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
pub enum IrErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    /// A scope error.
    #[error("scope error: {error}")]
    ScopeError {
        /// The kind of the scope error.
        #[source]
        #[from]
        error: ScopeErrorKind,
    },
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
        meta: CompileMeta,
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
}
