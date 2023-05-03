use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::ast;
use crate::ast::{Spanned, SpannedError};
use crate::parse::{Expectation, IntoExpectation, LexerMode, ResolveError, ResolveErrorKind};
use crate::SourceId;

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
#[derive(Debug, Clone, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum ParseErrorKind {
    #[error("{message}")]
    Custom { message: Box<str> },
    #[error("{error}")]
    ResolveError { error: ResolveErrorKind },
    #[error("Expected end of file, but got `{actual}`")]
    ExpectedEof { actual: ast::Kind },
    #[error("Unexpected end of file")]
    UnexpectedEof,
    #[error("Bad lexer mode `{actual}`, expected `{expected}`")]
    BadLexerMode {
        actual: LexerMode,
        expected: LexerMode,
    },
    #[error("Expected {expected}, but got {actual}")]
    Expected {
        actual: Expectation,
        expected: Expectation,
    },
    #[error("Unsupported `{what}`")]
    Unsupported { what: Expectation },
    #[error("Expected escape sequence")]
    ExpectedEscape,
    #[error("Unterminated string literal")]
    UnterminatedStrLit,
    #[error("Unterminated byte string literal")]
    UnterminatedByteStrLit,
    #[error("Unterminated character literal")]
    UnterminatedCharLit,
    #[error("Unterminated byte literal")]
    UnterminatedByteLit,
    #[error("Expected character literal to be closed")]
    ExpectedCharClose,
    #[error("Expected label or character")]
    ExpectedCharOrLabel,
    #[error("Expected byte literal to be closed")]
    ExpectedByteClose,
    #[error("Unexpected character `{c}`")]
    UnexpectedChar { c: char },
    #[error("Group required in expression to determine precedence")]
    PrecedenceGroupRequired,
    #[error("Number literal out of bounds `-9223372036854775808` to `9223372036854775807`")]
    BadNumberOutOfBounds,
    #[error("Unsupported field access")]
    BadFieldAccess,
    #[error("Expected close delimiter `{expected}`, but got `{actual}`")]
    ExpectedMacroCloseDelimiter {
        expected: ast::Kind,
        actual: ast::Kind,
    },
    #[error("Bad number literal")]
    BadNumber,
    #[error("Can only specify one attribute named `{name}`")]
    MultipleMatchingAttributes { name: &'static str },
    #[error("Missing source id `{source_id}`")]
    MissingSourceId { source_id: SourceId },
    #[error("Expected multiline comment to be terminated with a `*/`")]
    ExpectedMultilineCommentTerm,
}
