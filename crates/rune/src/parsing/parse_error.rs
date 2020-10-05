use crate::ast;
use crate::parsing::LexerMode;
use crate::shared::Description;
use crate::Spanned;
use runestick::Span;
use thiserror::Error;

error! {
    /// An error raised during parsing.
    #[derive(Debug, Clone, Copy)]
    pub struct ParseError {
        kind: ParseErrorKind,
    }
}

impl ParseError {
    /// Construct an expectation error.
    pub(crate) fn expected<T, E>(actual: T, expected: E) -> Self
    where
        T: Spanned,
        ast::Kind: From<T>,
        E: Description,
    {
        Self {
            span: actual.span(),
            kind: ParseErrorKind::Expected {
                expected: expected.description(),
                actual: ast::Kind::from(actual),
            },
        }
    }
}

/// Error when parsing.
#[derive(Debug, Clone, Copy, Error)]
#[allow(missing_docs)]
pub enum ParseErrorKind {
    /// Error raised when we expect and end-of-file but it didn't happen.
    #[error("expected end-of-file, but got token `{actual}`")]
    ExpectedEof {
        /// Kind of the token encountered instead of end-of-file.
        actual: ast::Kind,
    },
    /// Error raised when we encounter end-of-file but we didn't expect it.
    #[error("unexpected end-of-file")]
    UnexpectedEof,
    #[error("bad lexer mode `{mode}`, expected `{expected}`")]
    BadLexerMode {
        mode: LexerMode,
        expected: LexerMode,
    },
    /// An expectation error.
    #[error("expected {expected}, but got `{actual}`")]
    Expected {
        /// Description of what we expected.
        expected: &'static str,
        /// The actual kind seen.
        actual: ast::Kind,
    },
    /// The given item does not support an attribute, like `#[foo]`.
    #[error("item does not support attributes")]
    UnsupportedItemAttributes,
    /// The given item does not support a visibility modifier, like `pub`.
    #[error("item does not support visibility")]
    UnsupportedItemVisibility,
    /// When we try to use a visibility modifer for an expression.
    #[error("visibility modifier is not supported for expressions")]
    UnsupportedExprVisibility,
    /// Error encountered when we see a string escape sequence without a
    /// character being escaped.
    #[error("expected escape sequence")]
    ExpectedEscape,
    /// Expected a string close but didn't see it.
    #[error("unterminated string literal")]
    UnterminatedStrLit,
    /// Expected a byte string close but didn't see it.
    #[error("unterminated byte string literal")]
    UnterminatedByteStrLit,
    /// Encountered an unterminated character literal.
    #[error("unterminated character literal")]
    UnterminatedCharLit,
    /// Encountered an unterminated byte literal.
    #[error("unterminated byte literal")]
    UnterminatedByteLit,
    /// Expected a character to be closed.
    #[error("expected character literal to be closed")]
    ExpectedCharClose,
    /// Expected a byte to be closed.
    #[error("expected byte literal to be closed")]
    ExpectedByteClose,
    /// Expected a string template to be closed, but it wasn't.
    #[error("expected string template to be closed")]
    ExpectedTemplateClose,
    /// Encountered an unexpected token.
    #[error("token mismatch, expected `{expected}` but was `{actual}`")]
    TokenMismatch {
        /// The kind of the expected token we saw.
        expected: ast::Kind,
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// Encountered an unexpected character.
    #[error("unexpected character `{c}`")]
    UnexpectedChar {
        /// Character encountered.
        c: char,
    },
    /// Expression group required to break precedence.
    #[error("group required in expression to determine precedence")]
    PrecedenceGroupRequired,
    /// Attempt to read a slice which doesn't exist.
    #[error("tried to read bad slice from source")]
    BadSlice,
    /// Attempt to get a value for a synthetic identifier.
    #[error("tried to get bad synthetic identifier `{id}` for {kind}")]
    BadSyntheticId {
        /// The kind of id we tried to fetch.
        kind: &'static str,
        /// The identifier that was bad.
        id: usize,
    },
    /// Encountered a bad string escape sequence.
    #[error("bad escape sequence")]
    BadEscapeSequence,
    /// Tried to resolve an illegal number literal.
    #[error("number literal not valid")]
    BadNumberLiteral,
    /// Number out of bounds.
    #[error("number literal out of bounds `-9223372036854775808` to `9223372036854775807`")]
    BadNumberOutOfBounds,
    /// A bad character literal.
    #[error("bad character literal")]
    BadCharLiteral,
    /// A bad byte literal.
    #[error("bad byte literal")]
    BadByteLiteral,
    /// We tried to parse a unicode escape in a byte sequence.
    #[error("unicode escapes are not supported as a byte or byte string")]
    UnicodeEscapeNotSupported,
    /// Error when we encounter a bad unicode escape.
    #[error("bad unicode escape")]
    BadUnicodeEscape,
    /// Error when we encounter a bad byte escape in bounds.
    #[error(
        "this form of character escape may only be used with characters in the range [\\x00-\\x7f]"
    )]
    UnsupportedUnicodeByteEscape,
    /// Error when we encounter a bad byte escape in bounds.
    #[error(
        "this form of byte escape may only be used with characters in the range [\\x00-\\xff]"
    )]
    UnsupportedByteEscape,
    /// Error when we encounter a bad byte escape.
    #[error("bad byte escape")]
    BadByteEscape,
    /// When we encounter an invalid template literal.
    #[error("template expression unexpectedly ended")]
    UnexpectedExprEnd,
    /// When we encounter an unescaped closing brace `}`.
    #[error("closing braces must be escaped inside of templates with `\\}}`")]
    UnexpectedCloseBrace,
    /// When we encounter an expression that cannot be used in a chained manner.
    #[error("unsupported field access")]
    UnsupportedFieldAccess,
    /// Trying to use an expression as async when it's not supported.
    #[error("not supported as an async expression")]
    UnsupportedAsyncExpr,
    /// Expected a macro close delimiter.
    #[error("expected close delimiter `{expected}`, but got `{actual}`")]
    ExpectedMacroCloseDelimiter {
        /// The delimiter we expected.
        expected: ast::Kind,
        /// The delimiter we saw.
        actual: ast::Kind,
    },
    /// Encountered a position with attributes for which it is not supported.
    #[error("attributes not supported in this position")]
    AttributesNotSupported,
    /// Encountered when we expect inner attributes.
    #[error("expected inner attribute")]
    ExpectedInnerAttribute,
    #[error("item needs to be terminated by a semi-colon `;`")]
    ItemNeedsSemi,
    #[error("expected `while`, `for`, `loop` after a label")]
    UnsupportedLabel,
    #[error("expected block, or `fn` after `const`")]
    UnsupportedConst,
    #[error("expected block, closure, or `fn` after `async`")]
    UnsupportedAsync,
    #[error("bad number literal")]
    BadNumber,
    #[error("expected identifier for object key")]
    ExpectedObjectIdent,
    #[error("expected template lexer mode")]
    ExpectedTemplateMode,
}
