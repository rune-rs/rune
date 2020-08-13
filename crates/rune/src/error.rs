use crate::ast;
use crate::token::Kind;
use runestick::{Item, Meta, Span};
use std::fmt;
use std::io;
use thiserror::Error;

/// Result alias used by this frontend.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error capable of collecting all error types emitted by this crate.
#[derive(Debug, Error)]
pub enum Error {
    /// Source parse error.
    #[error("parse error")]
    ParseError(#[from] ParseError),
    /// Compiler error.
    #[error("compile error")]
    CompileError(#[from] CompileError),
    /// Configuration error.
    #[error("configuration error")]
    ConfigurationError(#[from] ConfigurationError),
    /// I/O error.
    #[error("I/O error")]
    Io(#[from] io::Error),
    /// Formatting error.
    #[error("formatting error")]
    Fmt(#[from] fmt::Error),
    /// Errors raised by the virtual machine.
    #[error("virtual machine error")]
    VmError(#[from] runestick::VmError),
    /// Errors raised when setting up context.
    #[error("context error")]
    ContextError(#[from] runestick::ContextError),
}

#[derive(Debug, Clone, Error)]
pub enum ConfigurationError {
    /// Tried to configure the compiler with an unsupported optimzation option.
    #[error("unsupported optimization option `{option}`")]
    UnsupportedOptimizationOption {
        /// The unsupported option.
        option: String,
    },
}

/// Error when parsing.
#[derive(Debug, Clone, Copy, Error)]
pub enum ParseError {
    /// Error raised when we encounter end-of-file but we didn't expect it.
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
    #[error("expected escape")]
    ExpectedStringEscape {
        /// Span that caused the error.
        span: Span,
    },
    /// Expected a string close but didn't see it.
    #[error("unterminated string literal")]
    UnterminatedStrLit {
        /// Span that caused the error.
        span: Span,
    },
    /// Encountered an unterminated character literal.
    #[error("unterminated character literal")]
    UnterminatedCharLit {
        /// The span of the unterminated literal.
        span: Span,
    },
    /// Encountered an unterminated byte literal.
    #[error("unterminated byte literal")]
    UnterminatedByteLit {
        /// The span of the unterminated literal.
        span: Span,
    },
    /// Expected a character to be closed.
    #[error("expected character literal to be closed")]
    ExpectedCharClose {
        /// Span that caused the error.
        span: Span,
    },
    /// Expected a byte to be closed.
    #[error("expected byte literal to be closed")]
    ExpectedByteClose {
        /// Span that caused the error.
        span: Span,
    },
    /// Expected a string template to be closed, but it wasn't.
    #[error("expected string template to be closed")]
    ExpectedTemplateClose {
        /// Span that caused the error.
        span: Span,
    },
    /// Error encountered when we see a character escape sequence without a
    /// character being escaped.
    #[error("expected character character")]
    ExpectedCharEscape {
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
    /// Expected a pattern but got something else.
    #[error("expected start of pattern but got `{actual}`")]
    ExpectedPatError {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected an expression but got something else.
    #[error("expected start of expression but got `{actual}`")]
    ExpectedExpr {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected an enum variant but got something else.
    #[error("expected enum variant but got `{actual}`")]
    ExpectedEnumVariant {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// When we expect to see a loop (typically after a label).
    #[error("expected loop but got `{actual}")]
    ExpectedLoop {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected a block expression but got something else.
    #[error("expected block expression but got `{actual}`")]
    ExpectedBlockExpr {
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
    ExpectedNumber {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected a byte, but got something else.
    #[error("expected byte but got `{actual}`")]
    ExpectedByte {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected a char, but got something else.
    #[error("expected char but got `{actual}`")]
    ExpectedChar {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// Expected a string, but got something else.
    #[error("expected string but got `{actual}`")]
    ExpectedString {
        /// Span that caused the error.
        span: Span,
        /// The actual token kind which was not a string.
        actual: Kind,
    },
    /// Expected an operator but got something else.
    #[error("expected operator (`+`, `-`, `/`, `*`) but got `{actual}`")]
    ExpectedOperator {
        /// The location of the unexpected operator.
        span: Span,
        /// The actual token that was encountered instead of an operator.
        actual: Kind,
    },
    /// Expected a boolean literal.
    #[error("expected `true` or `false` but got `{actual}`")]
    ExpectedBool {
        /// The location of the unexpected token.
        span: Span,
        /// The actual token that was encountered.
        actual: Kind,
    },
    /// Expected a valid object key.
    #[error("expected an object key (string or identifier) but got `{actual}`")]
    ExpectedLitObjectKey {
        /// The location of the unexpected token.
        span: Span,
        /// The actual token that was encountered.
        actual: Kind,
    },
    /// Trying to call an instance function consisting of a path.
    #[error("cannot call instance functions consisting of paths")]
    PathCallInstance {
        /// The location of the unexpected token.
        span: Span,
    },
    /// Expected a unary operator.
    #[error("expected unary operator (`!`) but got `{actual}`")]
    ExpectedUnaryOperator {
        /// The span that caused the error.
        span: Span,
        /// The actual token.
        actual: Kind,
    },
    /// Expression group required to break precedence.
    #[error("group required in expression to determine precedence")]
    PrecedenceGroupRequired {
        /// Span that caused the error.
        span: Span,
    },
    /// Attempt to read a slice which doesn't exist.
    #[error("tried to read bad slice from source `{span}`")]
    BadSlice {
        /// The slice we tried to read.
        span: Span,
    },
    /// Encountered a bad string escape sequence.
    #[error("bad escape sequence")]
    BadEscapeSequence {
        /// Span of the illegal escape sequence.
        span: Span,
    },
    /// Tried to resolve an illegal number literal.
    #[error("illegal number literal")]
    IllegalNumberLiteral {
        /// Span of the illegal number literal.
        span: Span,
    },
    /// A bad character literal.
    #[error("bad character literal")]
    BadCharLiteral {
        /// Span containing the bad character literal.
        span: Span,
    },
    /// A bad byte literal.
    #[error("bad byte literal")]
    BadByteLiteral {
        /// Span containing the bad byte literal.
        span: Span,
    },
    /// We tried to parse a unicode escape in a byte sequence.
    #[error("unicode escapes are not supported as a byte or byte string")]
    UnicodeEscapeNotSupported {
        /// Where the bad escape is.
        span: Span,
    },
    /// Error when we encounter a bad unicode escape.
    #[error("bad unicode escape")]
    BadUnicodeEscape {
        /// Where the bad escape is.
        span: Span,
    },
    /// Error when we encounter a bad byte escape in bounds.
    #[error(
        "this form of character escape may only be used with characters in the range [\\x00-\\x7f]"
    )]
    UnsupportedUnicodeByteEscape {
        /// Where the bad escape is.
        span: Span,
    },
    /// Error when we encounter a bad byte escape in bounds.
    #[error(
        "this form of byte escape may only be used with characters in the range [\\x00-\\xff]"
    )]
    UnsupportedByteEscape {
        /// Where the bad escape is.
        span: Span,
    },
    /// Error when we encounter a bad byte escape.
    #[error("bad byte escape")]
    BadByteEscape {
        /// Where the bad escape is.
        span: Span,
    },
    /// When we encounter an invalid template literal.
    #[error("invalid template literal")]
    InvalidTemplateLiteral {
        /// The span where the error occured.
        span: Span,
    },
    /// When we encounter an unescaped closing brace `}`.
    #[error("closing braces must be escaped inside of templates with `\\}}`")]
    UnexpectedCloseBrace {
        /// Where the brace was encountered.
        span: Span,
    },
}

impl ParseError {
    /// Get the span for the parse error.
    pub fn span(&self) -> Span {
        match *self {
            Self::UnexpectedEof { span, .. } => span,
            Self::ExpectedEof { span, .. } => span,
            Self::ExpectedStringEscape { span, .. } => span,
            Self::UnterminatedStrLit { span, .. } => span,
            Self::UnterminatedCharLit { span, .. } => span,
            Self::UnterminatedByteLit { span, .. } => span,
            Self::ExpectedCharEscape { span, .. } => span,
            Self::ExpectedCharClose { span, .. } => span,
            Self::ExpectedByteClose { span, .. } => span,
            Self::ExpectedTemplateClose { span, .. } => span,
            Self::TokenMismatch { span, .. } => span,
            Self::ExpectedPatError { span, .. } => span,
            Self::ExpectedExpr { span, .. } => span,
            Self::ExpectedEnumVariant { span, .. } => span,
            Self::ExpectedLoop { span, .. } => span,
            Self::ExpectedBlockExpr { span, .. } => span,
            Self::UnexpectedChar { span, .. } => span,
            Self::ExpectedNumber { span, .. } => span,
            Self::ExpectedByte { span, .. } => span,
            Self::ExpectedChar { span, .. } => span,
            Self::ExpectedString { span, .. } => span,
            Self::ExpectedOperator { span, .. } => span,
            Self::ExpectedBool { span, .. } => span,
            Self::ExpectedLitObjectKey { span, .. } => span,
            Self::PathCallInstance { span, .. } => span,
            Self::ExpectedUnaryOperator { span, .. } => span,
            Self::PrecedenceGroupRequired { span, .. } => span,
            Self::BadSlice { span, .. } => span,
            Self::BadEscapeSequence { span, .. } => span,
            Self::IllegalNumberLiteral { span, .. } => span,
            Self::BadCharLiteral { span, .. } => span,
            Self::BadByteLiteral { span, .. } => span,
            Self::UnicodeEscapeNotSupported { span, .. } => span,
            Self::BadUnicodeEscape { span, .. } => span,
            Self::UnsupportedUnicodeByteEscape { span, .. } => span,
            Self::UnsupportedByteEscape { span, .. } => span,
            Self::BadByteEscape { span, .. } => span,
            Self::InvalidTemplateLiteral { span, .. } => span,
            Self::UnexpectedCloseBrace { span, .. } => span,
        }
    }
}

/// Error when encoding AST.
#[derive(Debug, Error)]
pub enum CompileError {
    /// An internal encoder invariant was broken.
    #[error("internal compiler error: {msg}")]
    Internal {
        /// The message of the variant.
        msg: &'static str,
        /// Where the invariant was broken.
        span: Span,
    },
    /// Unit error from runestick encoding.
    #[error("unit construction error: {error}")]
    UnitError {
        /// Source error.
        #[from]
        error: runestick::CompilationUnitError,
    },
    /// Error for resolving values from source files.
    #[error("{error}")]
    ParseError {
        /// Source error.
        #[from]
        error: ParseError,
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
    #[error("missing variable `{name}`")]
    MissingLocal {
        /// Span where the error occured.
        span: Span,
        /// Name of the missing variable.
        name: String,
    },
    /// Error for moved local variables.
    #[error("variable `{name}` has been moved")]
    MovedLocal {
        /// Span where the error occured.
        span: Span,
        /// Name of the moved variable.
        name: String,
        /// Where it was moved.
        moved_at: Span,
    },
    /// Error for missing types.
    #[error("no type matching `{item}`")]
    MissingType {
        /// Span where the error occured.
        span: Span,
        /// Name of the missing type.
        item: Item,
    },
    /// Tried to use a module that was missing.
    #[error("missing module `{module}`")]
    MissingModule {
        /// The span of the missing module.
        span: Span,
        /// The name of the missing module.
        module: Item,
    },
    /// A specific label is missing.
    #[error("label not found in scope")]
    MissingLabel {
        /// The span of the missing label.
        span: Span,
    },
    /// Encountered a unary operator we can't encode.
    #[error("unsupported unary operator `{op}`")]
    UnsupportedUnaryOp {
        /// The span of the illegal operator use.
        span: Span,
        /// The operator.
        op: ast::UnaryOp,
    },
    /// Encountered a binary operator we can't encode.
    #[error("unsupported binary operator `{op}`")]
    UnsupportedBinaryOp {
        /// The span of the illegal call.
        span: Span,
        /// The operator.
        op: ast::BinOp,
    },
    /// Cannot crate object literal of the given type.
    #[error("type `{item}` is not an object")]
    UnsupportedLitObject {
        /// The span of the unsupported object.
        span: Span,
        /// The path to the unsupported object.
        item: Item,
    },
    /// Key is not present in the given type literal.
    #[error("missing field `{field}` in declaration of `{item}`")]
    LitObjectMissingField {
        /// The span of the unsupported object.
        span: Span,
        /// They key that didn't exist.
        field: String,
        /// The related item.
        item: Item,
    },
    /// Key is not present in the given type literal.
    #[error("`{field}` is not a field in `{item}`")]
    LitObjectNotField {
        /// The span of the unsupported object.
        span: Span,
        /// They key that is not a field.
        field: String,
        /// The related item.
        item: Item,
    },
    /// When we encounter an expression that cannot be assigned to.
    #[error("cannot assign to expression")]
    UnsupportedAssignExpr {
        /// The thing being assigned to.
        span: Span,
    },
    /// Unsupported assignment operator.
    #[error("unsupported operator `{op}` in assignment")]
    UnsupportedAssignBinOp {
        /// The assign expression.
        span: Span,
        /// The unsupported operator.
        op: ast::BinOp,
    },
    /// When we encounter an expression that doesn't have a stack location and
    /// can't be referenced.
    #[error("cannot take reference of expression")]
    UnsupportedRef {
        /// The thing we are taking the reference of.
        span: Span,
    },
    /// Await has been used in a position where it's not supported.
    #[error("`await` expression is not supported in this location")]
    UnsupportedAwait {
        /// The location of the await.
        span: Span,
    },
    /// Using a pattern that is not supported in a select.
    #[error("unsupported select pattern")]
    UnsupportedSelectPattern {
        /// The span of the pattern.
        span: Span,
    },
    /// Unsupported field access.
    #[error("unsupported field access")]
    UnsupportedFieldAccess {
        /// The field access expression.
        span: Span,
    },
    /// A meta item that is not supported in the given pattern position.
    #[error("wrong number of arguments, expected `{expected}` but got `{actual}`")]
    UnsupportedArgumentCount {
        /// The span which the error occured.
        span: Span,
        /// The meta item we tried to use as a pattern.
        meta: Meta,
        /// The expected number of arguments.
        expected: usize,
        /// The actual number of arguments.
        actual: usize,
    },
    /// A meta item that is not supported in the given pattern position.
    #[error("`{meta}` is not supported in a pattern like this")]
    UnsupportedMetaPattern {
        /// The meta item we tried to use as a pattern.
        meta: Meta,
        /// The span which the error occured.
        span: Span,
    },
    /// The pattern is not supported.
    #[error("item is not supported in a pattern")]
    UnsupportedPattern {
        /// Span where the error occured.
        span: Span,
    },
    /// Error raised when trying to use a break outside of a loop.
    #[error("break expressions cannot be used as a value")]
    BreakOutsideOfLoop {
        /// The span of the illegal break.
        span: Span,
    },
    /// An error raised when attempting to return locally created references
    /// from a function.
    #[error("cannot return locally created references")]
    ReturnLocalReferences {
        /// The span which we try to return from.
        block: Span,
        /// The span at which we tried to return.
        span: Span,
        /// The references we tried to return.
        references_at: Vec<Span>,
    },
    /// Attempting to use a float in a match pattern.
    #[error("floating point numbers cannot be used in patterns")]
    MatchFloatInPattern {
        /// Where the float was used.
        span: Span,
    },
    /// Attempting to create an object with a duplicate object key.
    #[error("duplicate key in literal object")]
    DuplicateObjectKey {
        /// Where the key was re-defined.
        span: Span,
        /// Where the object key exists previously.
        existing: Span,
        /// The object being defined.
        object: Span,
    },
    /// Attempt to call something that is not a function.
    #[error("cannot be called as a function")]
    NotFunction {
        /// The span of the unsupported function call.
        span: Span,
    },
}

impl CompileError {
    /// Construct an internal error.
    ///
    /// This should be used for programming invariants of the encoder which are
    /// broken for some reason.
    pub fn internal(msg: &'static str, span: Span) -> Self {
        Self::Internal { msg, span }
    }
}

impl CompileError {
    /// Get the span for the error.
    pub fn span(&self) -> Span {
        match *self {
            Self::UnitError { .. } => Span::default(),
            Self::Internal { span, .. } => span,
            Self::ParseError { error, .. } => error.span(),
            Self::VariableConflict { span, .. } => span,
            Self::MissingLocal { span, .. } => span,
            Self::MovedLocal { span, .. } => span,
            Self::MissingType { span, .. } => span,
            Self::MissingModule { span, .. } => span,
            Self::MissingLabel { span, .. } => span,
            Self::UnsupportedRef { span, .. } => span,
            Self::UnsupportedAwait { span, .. } => span,
            Self::UnsupportedUnaryOp { span, .. } => span,
            Self::UnsupportedBinaryOp { span, .. } => span,
            Self::UnsupportedLitObject { span, .. } => span,
            Self::UnsupportedAssignExpr { span, .. } => span,
            Self::UnsupportedAssignBinOp { span, .. } => span,
            Self::UnsupportedSelectPattern { span, .. } => span,
            Self::UnsupportedFieldAccess { span, .. } => span,
            Self::UnsupportedArgumentCount { span, .. } => span,
            Self::UnsupportedMetaPattern { span, .. } => span,
            Self::UnsupportedPattern { span, .. } => span,
            Self::BreakOutsideOfLoop { span, .. } => span,
            Self::ReturnLocalReferences { span, .. } => span,
            Self::MatchFloatInPattern { span, .. } => span,
            Self::DuplicateObjectKey { span, .. } => span,
            Self::NotFunction { span, .. } => span,
            Self::LitObjectMissingField { span, .. } => span,
            Self::LitObjectNotField { span, .. } => span,
        }
    }
}
