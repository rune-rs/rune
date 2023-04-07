use crate::ast::{Spanned, SpannedError};
use crate::macros::{Storage, SyntheticId, SyntheticKind};
use crate::parse::{Expectation, IntoExpectation};
use crate::Sources;
use thiserror::Error;

error! {
    /// An error during resolving.
    #[derive(Debug, Clone)]
    pub struct ResolveError {
        kind: ResolveErrorKind,
    }
}

impl ResolveError {
    /// Construct an expectation error.
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

impl From<ResolveError> for SpannedError {
    fn from(error: ResolveError) -> Self {
        SpannedError::new(error.span, error.kind)
    }
}

/// The kind of a resolve error.
#[derive(Debug, Clone, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum ResolveErrorKind {
    #[error("{message}")]
    Custom { message: Box<str> },
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

/// A resolve context.
#[derive(Clone, Copy)]
pub struct ResolveContext<'a> {
    /// Sources to use.
    pub(crate) sources: &'a Sources,
    /// Storage to use in resolve context.
    pub(crate) storage: &'a Storage,
}

/// A type that can be resolved to an internal value based on a source.
pub trait Resolve<'a> {
    /// The output type being resolved into.
    type Output: 'a;

    /// Resolve the value from parsed AST.
    fn resolve(&self, ctx: ResolveContext<'a>) -> Result<Self::Output, ResolveError>;
}
