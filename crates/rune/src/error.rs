use crate::token::{Kind, Span};
use thiserror::Error;

/// Result alias used by this frontend.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error with an associated span.
pub trait SpannedError {
    /// Access the span of the error.
    fn span(&self) -> Span;
}

/// Error capable of collecting all error types.
#[derive(Debug, Error)]
pub enum Error {
    /// Source resolve error.
    #[error("resolve error")]
    ResolveError(#[from] ResolveError),
    /// Source parse error.
    #[error("parse error")]
    ParseError(#[from] ParseError),
    /// Source encode error.
    #[error("encode error")]
    EncodeError(#[from] EncodeError),
}

impl SpannedError for Error {
    fn span(&self) -> Span {
        match self {
            Self::ResolveError(e) => e.span(),
            Self::ParseError(e) => e.span(),
            Self::EncodeError(e) => e.span(),
        }
    }
}

/// Error raised when resolving a value.
#[derive(Debug, Clone, Copy, Error)]
pub enum ResolveError {
    /// Encountered a bad string escape sequence.
    #[error("bad string escape sequence character `{c}`")]
    BadStringEscapeSequence {
        /// Span of the illegal escape sequence.
        span: Span,
        /// The illegal character.
        c: char,
    },
    /// Tried to resolve an illegal number literal.
    #[error("illegal number literal")]
    IllegalNumberLiteral {
        /// Span of the illegal number literal.
        span: Span,
    },
}

impl SpannedError for ResolveError {
    fn span(&self) -> Span {
        match *self {
            Self::BadStringEscapeSequence { span, .. } => span,
            Self::IllegalNumberLiteral { span, .. } => span,
        }
    }
}

/// Error when parsing.
#[derive(Debug, Clone, Copy, Error)]
pub enum ParseError {
    /// Error raised when we encounter enf-of-file but we didn't expect it.
    #[error("unexpected end-of-file")]
    UnexpectedEof {
        /// Span that caused the error.
        span: Span,
    },
    /// Error raised when we expect and end-of-file but it didn't happen.
    #[error("expected end of file, but encountered `{actual}`")]
    ExpectedEof {
        /// Span that caused the error.
        span: Span,
        /// Kind of the token encountered instead of end-of-file.
        actual: Kind,
    },
    /// Error encountered when we see a string escape sequence without a
    /// character being escaped.
    #[error("expected escape character")]
    ExpectedStringEscape {
        /// Span that caused the error.
        span: Span,
    },
    /// Expected a string close but didn't see it.
    #[error("expected string to be closed")]
    ExpectedStringClose {
        /// Span that caused the error.
        span: Span,
    },
    /// Encountered an unexpected token.
    #[error("token mismatch, expected `{expected}` but was `{actual}`")]
    TokenMismatch {
        /// Span that caused the error.
        span: Span,
        /// The kind of the expected token we saw.
        expected: Kind,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected an expression but got something else.
    #[error("expected expression but got `{actual}`")]
    ExpectedExprError {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected a block expression but got something else.
    #[error("expected block expression but got `{actual}`")]
    ExpectedBlockExprError {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Encountered an unexpected character.
    #[error("unexpected character `{c}`")]
    UnexpectedChar {
        /// Span that caused the error.
        span: Span,
        /// Character encountered.
        c: char,
    },
    /// Expected a number, but got something else.
    #[error("expected number but got `{actual}`")]
    ExpectedNumberError {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected a string, but got something else.
    #[error("expected string but got `{actual}`")]
    ExpectedStringError {
        /// Span that caused the error.
        span: Span,
        /// The actual token kind which was not a string.
        actual: Kind,
    },
    /// Expected an operator but got something else.
    #[error("expected operator (`+`, `-`, `/`, `*`) but got `{actual}`")]
    ExpectedOperatorError {
        /// The location of the unexpected operator.
        span: Span,
        /// The actual token that was encountered instead of an operator.
        actual: Kind,
    },
    /// Expected a boolean literal.
    #[error("expected `true` or `false` but got `{actual}`")]
    ExpectedBoolError {
        /// The location of the unexpected token.
        span: Span,
        /// The actual token that was encountered.
        actual: Kind,
    },
}

impl SpannedError for ParseError {
    fn span(&self) -> Span {
        match *self {
            Self::UnexpectedEof { span, .. } => span,
            Self::ExpectedEof { span, .. } => span,
            Self::ExpectedStringEscape { span, .. } => span,
            Self::ExpectedStringClose { span, .. } => span,
            Self::TokenMismatch { span, .. } => span,
            Self::ExpectedExprError { span, .. } => span,
            Self::ExpectedBlockExprError { span, .. } => span,
            Self::UnexpectedChar { span, .. } => span,
            Self::ExpectedNumberError { span, .. } => span,
            Self::ExpectedStringError { span, .. } => span,
            Self::ExpectedOperatorError { span, .. } => span,
            Self::ExpectedBoolError { span, .. } => span,
        }
    }
}

/// Error when encoding AST.
#[derive(Debug, Error)]
pub enum EncodeError {
    /// Unit error from ST encoding.
    #[error("unit construction error")]
    UnitError {
        /// Source error.
        #[from]
        error: st::UnitError,
    },
    /// Error for resolving values from source files.
    #[error("resolve error")]
    ResolveError {
        /// Source error.
        #[from]
        error: ResolveError,
    },
    /// Error for variable conflicts.
    #[error("variable `{name}` conflicts")]
    VariableConflict {
        /// Span where the error occured.
        span: Span,
        /// Name of the conflicting variable.
        name: String,
        /// The span where the variable was already present.
        existing_span: Span,
    },
    /// Error for missing local variables.
    #[error("missing local variable `{name}`")]
    MissingLocal {
        /// Span where the error occured.
        span: Span,
        /// Name of the missing local.
        name: String,
    },
    /// Tried to use a module that was missing.
    #[error("missing module `{module}`")]
    MissingModule {
        /// The span of the missing module.
        span: Span,
        /// The name of the missing module.
        module: st::ItemPath,
    },
    /// Encountered expression that must be closed.
    #[error("expression must be closed")]
    ExprNotClosed {
        /// Span of the expression that was not closed.
        span: Span,
    },
}

impl SpannedError for EncodeError {
    fn span(&self) -> Span {
        match *self {
            Self::UnitError { .. } => Span::default(),
            Self::ResolveError { error, .. } => error.span(),
            Self::VariableConflict { span, .. } => span,
            Self::MissingLocal { span, .. } => span,
            Self::MissingModule { span, .. } => span,
            Self::ExprNotClosed { span, .. } => span,
        }
    }
}
