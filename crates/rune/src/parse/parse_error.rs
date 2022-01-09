use crate::ast;
use crate::ast::{Spanned, SpannedError};
use crate::parse::{Expectation, IntoExpectation, LexerMode, ResolveError, ResolveErrorKind};
use crate::SourceId;
use thiserror::Error;

error! {
    /// An error raised during parsing.
    #[derive(Debug, Clone)]
    pub struct ParseError {
        kind: ParseErrorKind,
    }

    impl From<ResolveError>;
}

impl ParseError {
    /// Construct an expectation error.
    pub(crate) fn expected<A, E>(actual: A, expected: E) -> Self
    where
        A: IntoExpectation + Spanned,
        E: IntoExpectation,
    {
        Self::new(
            actual.span(),
            ParseErrorKind::Expected {
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
            ParseErrorKind::Unsupported {
                what: what.into_expectation(),
            },
        )
    }
}

impl From<ParseError> for SpannedError {
    fn from(error: ParseError) -> Self {
        SpannedError::new(error.span, *error.kind)
    }
}

/// Error when parsing.
#[derive(Debug, Clone, Copy, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ParseErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("{error}")]
    ResolveError { error: ResolveErrorKind },
    #[error("expected end of file, but got `{actual}`")]
    ExpectedEof { actual: ast::Kind },
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("bad lexer mode `{actual}`, expected `{expected}`")]
    BadLexerMode {
        actual: LexerMode,
        expected: LexerMode,
    },
    #[error("expected {expected}, but got {actual}")]
    Expected {
        actual: Expectation,
        expected: Expectation,
    },
    #[error("{what} is not supported")]
    Unsupported { what: Expectation },
    #[error("expected escape sequence")]
    ExpectedEscape,
    #[error("unterminated string literal")]
    UnterminatedStrLit,
    #[error("unterminated byte string literal")]
    UnterminatedByteStrLit,
    #[error("unterminated character literal")]
    UnterminatedCharLit,
    #[error("unterminated byte literal")]
    UnterminatedByteLit,
    #[error("expected character literal to be closed")]
    ExpectedCharClose,
    #[error("expected label or character")]
    ExpectedCharOrLabel,
    #[error("expected byte literal to be closed")]
    ExpectedByteClose,
    #[error("unexpected character `{c}`")]
    UnexpectedChar { c: char },
    #[error("group required in expression to determine precedence")]
    PrecedenceGroupRequired,
    #[error("number literal out of bounds `-9223372036854775808` to `9223372036854775807`")]
    BadNumberOutOfBounds,
    #[error("unsupported field access")]
    BadFieldAccess,
    #[error("expected close delimiter `{expected}`, but got `{actual}`")]
    ExpectedMacroCloseDelimiter {
        expected: ast::Kind,
        actual: ast::Kind,
    },
    #[error("bad number literal")]
    BadNumber,
    #[error("can only specify one attribute named `{name}`")]
    MultipleMatchingAttributes { name: &'static str },
    #[error("missing source id `{source_id}`")]
    MissingSourceId { source_id: SourceId },
    #[error("expected multiline comment to be terminated with a `*/`")]
    ExpectedMultilineCommentTerm,
}
