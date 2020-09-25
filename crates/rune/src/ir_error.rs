use crate::ir_value::IrValue;
use crate::{QueryError, QueryErrorKind, Spanned};
use runestick::{AccessError, CompileMeta, Span, TypeInfo, TypeOf};
use std::error;
use std::fmt;
use thiserror::Error;

/// An error raised during compiling.
#[derive(Debug)]
pub struct IrError {
    span: Span,
    kind: IrErrorKind,
}

impl IrError {
    /// Construct a new error.
    pub fn new<S, E>(spanned: S, err: E) -> Self
    where
        S: Spanned,
        IrErrorKind: From<E>,
    {
        Self {
            span: spanned.span(),
            kind: IrErrorKind::from(err),
        }
    }

    /// Get the kind of the error.
    pub fn kind(&self) -> &IrErrorKind {
        &self.kind
    }

    /// Convert into the kind of the error.
    pub fn into_kind(self) -> IrErrorKind {
        self.kind
    }

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

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl error::Error for IrError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

impl Spanned for IrError {
    /// Get the span for the parse error.
    fn span(&self) -> Span {
        self.span
    }
}

impl From<QueryError> for IrError {
    fn from(error: QueryError) -> Self {
        Self {
            span: error.span(),
            kind: IrErrorKind::QueryError {
                error: Box::new(error.into_kind()),
            },
        }
    }
}

/// Error when encoding AST.
#[derive(Debug, Error)]
pub enum IrErrorKind {
    /// A custom error.
    #[error("{0}")]
    Custom(&'static str),
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
