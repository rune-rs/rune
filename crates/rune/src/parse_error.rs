use crate::ast;
use crate::Spanned;
use runestick::Span;
use std::error;
use std::fmt;
use thiserror::Error;

/// An error raised during parsing.
#[derive(Debug, Clone, Copy)]
pub struct ParseError {
    span: Span,
    kind: ParseErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

impl ParseError {
    /// Construct a new parse error.
    pub(crate) fn new<S, E>(spanned: S, err: E) -> Self
    where
        S: Spanned,
        ParseErrorKind: From<E>,
    {
        Self {
            span: spanned.span(),
            kind: ParseErrorKind::from(err),
        }
    }

    /// Get kind of the parse error.
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }

    /// Get kind of the parse error.
    pub fn into_kind(self) -> ParseErrorKind {
        self.kind
    }
}

impl Spanned for ParseError {
    /// Get the span for the parse error.
    fn span(&self) -> Span {
        self.span
    }
}

/// Error when parsing.
#[derive(Debug, Clone, Copy, Error)]
pub enum ParseErrorKind {
    /// Error raised when we encounter end-of-file but we didn't expect it.
    #[error("unexpected end-of-file")]
    UnexpectedEof,
    /// Error raised when we expect and end-of-file but it didn't happen.
    #[error("expected end of input, but encountered `{actual}`")]
    ExpectedEof {
        /// Kind of the token encountered instead of end-of-file.
        actual: ast::Kind,
    },
    /// Error raised when we expect a declaration.
    #[error("expected declaration `fn`, `mod`, `struct`, `enum`, or `use`. got `{actual}`.")]
    ExpectedItem {
        /// Kind of the token encountered instead of a declaration.
        actual: ast::Kind,
    },
    /// Expected use import but found something else.
    #[error("expected import component but found `{actual}`")]
    ExpectedItemUseImportComponent {
        /// The actual token kind.
        actual: ast::Kind,
    },
    /// Error encountered when we see a string escape sequence without a
    /// character being escaped.
    #[error("expected escape")]
    ExpectedStringEscape,
    /// Expected a string close but didn't see it.
    #[error("unterminated string literal")]
    UnterminatedStrLit,
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
    /// Expected a pattern but got something else.
    #[error("expected start of pattern but got `{actual}`")]
    ExpectedPatError {
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// Expected an expression but got something else.
    #[error("expected start of expression but got `{actual}`")]
    ExpectedExpr {
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// When we expect to see a loop (typically after a label).
    #[error("expected loop but got `{actual}")]
    ExpectedLoop {
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// Encountered an unexpected character.
    #[error("unexpected character `{c}`")]
    UnexpectedChar {
        /// Character encountered.
        c: char,
    },
    /// Expected a number, but got something else.
    #[error("expected number but got `{actual}`")]
    ExpectedNumber {
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// Expected a byte, but got something else.
    #[error("expected byte but got `{actual}`")]
    ExpectedByte {
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// Expected a char, but got something else.
    #[error("expected char but got `{actual}`")]
    ExpectedChar {
        /// The kind of the actual token we saw.
        actual: ast::Kind,
    },
    /// Expected a string, but got something else.
    #[error("expected string but got `{actual}`")]
    ExpectedString {
        /// The actual token kind which was not a string.
        actual: ast::Kind,
    },
    /// Expected a boolean literal.
    #[error("expected `true` or `false` but got `{actual}`")]
    ExpectedBool {
        /// The actual token that was encountered.
        actual: ast::Kind,
    },
    /// Expected a valid object key.
    #[error("expected an object key (string or identifier) but got `{actual}`")]
    ExpectedLitObjectKey {
        /// The actual token that was encountered.
        actual: ast::Kind,
    },
    /// Expected a unary operator.
    #[error("expected unary operator (`!`) but got `{actual}`")]
    ExpectedUnaryOperator {
        /// The actual token.
        actual: ast::Kind,
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
    #[error("invalid template literal")]
    InvalidTemplateLiteral,
    /// When we encounter an unescaped closing brace `}`.
    #[error("closing braces must be escaped inside of templates with `\\}}`")]
    UnexpectedCloseBrace,
    /// When we encounter an expression that cannot be used in a chained manner.
    #[error("unsupported field access")]
    UnsupportedFieldAccess,
    /// Trying to use a token that is not supported as a function argument.
    #[error("not supported as a function or closure argument")]
    ExpectedFunctionArgument,
    /// Trying to use an expression as async when it's not supported.
    #[error("not supported as an async expression")]
    UnsupportedAsyncExpr,
    /// Expected a macro delimiter.
    #[error("expected delimiter, `(`, `[`, or `{{`, but got `{actual}`")]
    ExpectedMacroDelimiter {
        /// What we actually saw.
        actual: ast::Kind,
    },
    /// Expected a macro close delimiter.
    #[error("expected close delimiter `{expected}`, but got `{actual}`")]
    ExpectedMacroCloseDelimiter {
        /// The delimiter we expected.
        expected: ast::Kind,
        /// The delimiter we saw.
        actual: ast::Kind,
    },
    /// Expected a block semicolon which is needed for the kind of expression.
    #[error("expected expression to be terminated by a semicolon `;`")]
    ExpectedBlockSemiColon {
        /// The following expression.
        followed_span: Span,
    },
}
