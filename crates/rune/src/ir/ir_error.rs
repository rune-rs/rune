use crate::ir::IrValue;
use crate::shared::{Internal, ScopeError, ScopeErrorKind};
use crate::{QueryError, QueryErrorKind, Spanned};
use runestick::{AccessError, CompileMeta, Span, TypeInfo, TypeOf};
use thiserror::Error;

error! {
    /// An error raised during compiling.
    #[derive(Debug)]
    pub struct IrError {
        span: Span,
        kind: IrErrorKind,
    }

    impl From<QueryError>;
    impl From<ScopeError>;
}

impl From<Internal> for IrError {
    fn from(error: Internal) -> Self {
        Self {
            span: error.span(),
            kind: IrErrorKind::Internal {
                message: error.message(),
            },
        }
    }
}

impl IrError {
    /// Construct a custom error.
    pub fn custom<S>(spanned: S, message: &'static str) -> Self
    where
        S: Spanned,
    {
        Self::new(spanned, IrErrorKind::Custom(message))
    }

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
        move |error| Self::new(spanned, IrErrorKind::AccessError { error })
    }
}

/// Error when encoding AST.
#[derive(Debug, Error)]
pub enum IrErrorKind {
    /// A custom error.
    #[error("{0}")]
    Custom(&'static str),
    /// Internal compiler error.
    #[error("internal error: {message}")]
    Internal {
        /// Message of the error.
        message: &'static str,
    },
    /// Encountered an expression that is not supported as a constant
    /// expression.
    #[error("not a supported constant expression")]
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
    /// A scope error.
    #[error("scope error: {error}")]
    ScopeError {
        /// The kind of the scope error.
        #[source]
        error: Box<ScopeErrorKind>,
    },
    /// An access error raised during compilation.
    #[error("access error: {error}")]
    AccessError {
        /// The source error.
        #[source]
        error: AccessError,
    },
    /// An access error raised during queries.
    #[error("query error: {error}")]
    QueryError {
        /// The source error.
        #[source]
        error: Box<QueryErrorKind>,
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
    /// Error raised when trying to use a break outside of a loop.
    #[error("break outside of supported loop")]
    BreakOutsideOfLoop,
}
