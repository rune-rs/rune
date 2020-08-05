use crate::ast;
use crate::token::Kind;
use st::unit::Span;
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
    /// Compiler error.
    #[error("compile error")]
    CompileError(#[from] CompileError),
    /// Configuration error.
    #[error("configuration error")]
    ConfigurationError(#[from] ConfigurationError),
}

impl SpannedError for Error {
    fn span(&self) -> Span {
        match self {
            Self::ResolveError(e) => e.span(),
            Self::ParseError(e) => e.span(),
            Self::CompileError(e) => e.span(),
            Self::ConfigurationError(..) => Span::empty(),
        }
    }
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

/// Error raised when resolving a value.
#[derive(Debug, Clone, Copy, Error)]
pub enum ResolveError {
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
    BadCharacterLiteral {
        /// Span containing the bad character literal.
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
    BadByteEscapeBounds {
        /// Where the bad escape is.
        span: Span,
    },
    /// Error when we encounter a bad byte escape.
    #[error("bad byte escape")]
    BadByteEscape {
        /// Where the bad escape is.
        span: Span,
    },
}

impl SpannedError for ResolveError {
    fn span(&self) -> Span {
        match *self {
            Self::BadSlice { span, .. } => span,
            Self::BadEscapeSequence { span, .. } => span,
            Self::IllegalNumberLiteral { span, .. } => span,
            Self::BadCharacterLiteral { span, .. } => span,
            Self::BadUnicodeEscape { span, .. } => span,
            Self::BadByteEscapeBounds { span, .. } => span,
            Self::BadByteEscape { span, .. } => span,
        }
    }
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
    #[error("expected string character")]
    ExpectedStringEscape {
        /// Span that caused the error.
        span: Span,
    },
    /// Expected a string close but didn't see it.
    #[error("expected string literal to be closed")]
    ExpectedStringClose {
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
    /// Expected a char close but didn't see it.
    #[error("expected char literal to be closed")]
    ExpectedCharClose {
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
    ExpectedExprError {
        /// Span that caused the error.
        span: Span,
        /// The kind of the actual token we saw.
        actual: Kind,
    },
    /// When we expect to see a loop (typically after a label).
    #[error("expected loop but got `{actual}")]
    ExpectedLoopError {
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
    /// Expected a char, but got something else.
    #[error("expected char but got `{actual}`")]
    ExpectedCharError {
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
    /// Trying to call an instance function consisting of a path.
    #[error("cannot call instance functions consisting of paths")]
    PathCallInstanceError {
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
    /// Trying to define multiple fallback branches.
    #[error("multiple distinct fallback branches are defined")]
    MatchMultipleFallbackBranches {
        /// The current branch we are trying to define.
        span: Span,
        /// The existing branch we have already defined.
        existing: Span,
    },
    /// A match branch that will never be reached.
    #[error("match branch will never be reached")]
    MatchNeverReached {
        /// The span of the branch.
        span: Span,
        /// The span of the existing branch.
        existing: Span,
    },
    /// Expression group required to break precedence.
    #[error("group required in expression to determine precedence")]
    PrecedenceGroupRequired {
        /// Span that caused the error.
        span: Span,
    },
}

impl SpannedError for ParseError {
    fn span(&self) -> Span {
        match *self {
            Self::UnexpectedEof { span, .. } => span,
            Self::ExpectedEof { span, .. } => span,
            Self::ExpectedStringEscape { span, .. } => span,
            Self::ExpectedCharEscape { span, .. } => span,
            Self::ExpectedStringClose { span, .. } => span,
            Self::ExpectedCharClose { span, .. } => span,
            Self::TokenMismatch { span, .. } => span,
            Self::ExpectedPatError { span, .. } => span,
            Self::ExpectedExprError { span, .. } => span,
            Self::ExpectedLoopError { span, .. } => span,
            Self::ExpectedBlockExprError { span, .. } => span,
            Self::UnexpectedChar { span, .. } => span,
            Self::ExpectedNumberError { span, .. } => span,
            Self::ExpectedCharError { span, .. } => span,
            Self::ExpectedStringError { span, .. } => span,
            Self::ExpectedOperatorError { span, .. } => span,
            Self::ExpectedBoolError { span, .. } => span,
            Self::PathCallInstanceError { span, .. } => span,
            Self::ExpectedUnaryOperator { span, .. } => span,
            Self::MatchMultipleFallbackBranches { span, .. } => span,
            Self::MatchNeverReached { span, .. } => span,
            Self::PrecedenceGroupRequired { span, .. } => span,
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
    /// Unit error from ST encoding.
    #[error("unit construction error: {error}")]
    UnitError {
        /// Source error.
        #[from]
        error: st::CompilationUnitError,
    },
    /// Error for resolving values from source files.
    #[error("resolve error: {error}")]
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
        module: st::Item,
    },
    /// A specific label is missing.
    #[error("label not found in scope")]
    MissingLabel {
        /// The span of the missing label.
        span: Span,
    },
    /// Encountered a binary operator we can't encode.
    #[error("unsupported binary operator `{op}`")]
    UnsupportedBinaryOp {
        /// The span of the illegal call.
        span: Span,
        /// The operator.
        op: ast::BinOp,
    },
    /// Encountered a unary operator we can't encode.
    #[error("unsupported unary operator `{op}`")]
    UnsupportedUnaryOp {
        /// The span of the illegal operator use.
        span: Span,
        /// The operator.
        op: ast::UnaryOp,
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
    /// Attempt to assign a value to return.
    #[error("a return does not produce a value that can be used")]
    ReturnDoesNotProduceValue {
        /// The block in which it was used.
        block: Span,
        /// The span in which it was used.
        span: Span,
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

impl SpannedError for CompileError {
    fn span(&self) -> Span {
        match *self {
            Self::UnitError { .. } => Span::default(),
            Self::Internal { span, .. } => span,
            Self::ResolveError { error, .. } => error.span(),
            Self::VariableConflict { span, .. } => span,
            Self::MissingLocal { span, .. } => span,
            Self::MissingModule { span, .. } => span,
            Self::MissingLabel { span, .. } => span,
            Self::UnsupportedRef { span, .. } => span,
            Self::UnsupportedUnaryOp { span, .. } => span,
            Self::UnsupportedBinaryOp { span, .. } => span,
            Self::UnsupportedAssignExpr { span, .. } => span,
            Self::UnsupportedAssignBinOp { span, .. } => span,
            Self::BreakOutsideOfLoop { span, .. } => span,
            Self::ReturnLocalReferences { span, .. } => span,
            Self::ReturnDoesNotProduceValue { span, .. } => span,
            Self::MatchFloatInPattern { span, .. } => span,
            Self::DuplicateObjectKey { span, .. } => span,
        }
    }
}
