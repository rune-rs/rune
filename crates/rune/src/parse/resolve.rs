use crate::ast::{Spanned, SpannedError};
use crate::macros::Storage;
use crate::shared::Description;
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
        A: Description + Spanned,
        E: Description,
    {
        Self::new(
            actual.span(),
            ResolveErrorKind::Expected {
                actual: actual.description(),
                expected: expected.description(),
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
#[derive(Debug, Clone, Copy, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ResolveErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("expected {expected}, but got `{actual}`")]
    Expected {
        actual: &'static str,
        expected: &'static str,
    },
    #[error("tried to read bad slice from source")]
    BadSlice,
    #[error("tried to get bad synthetic identifier `{id}` for {kind}")]
    BadSyntheticId { kind: &'static str, id: usize },
    #[error("bad escape sequence")]
    BadEscapeSequence,
    #[error("bad unicode escape")]
    BadUnicodeEscape,
    #[error(
        "this form of character escape may only be used with characters in the range [\\x00-\\x7f]"
    )]
    BadHexEscapeChar,
    #[error(
        "this form of byte escape may only be used with characters in the range [\\x00-\\xff]"
    )]
    BadHexEscapeByte,
    #[error("bad byte escape")]
    BadByteEscape,
    #[error("bad character literal")]
    BadCharLiteral,
    #[error("bad byte literal")]
    BadByteLiteral,
    #[error("unicode escapes are not supported as a byte or byte string")]
    BadUnicodeEscapeInByteString,
    #[error("number literal not valid")]
    BadNumberLiteral,
}

/// A type that can be resolved to an internal value based on a source.
pub trait Resolve<'a> {
    /// The output type being resolved into.
    type Output: 'a;

    /// Resolve the value from parsed AST.
    fn resolve(
        &self,
        storage: &'a Storage,
        sources: &'a Sources,
    ) -> Result<Self::Output, ResolveError>;
}
