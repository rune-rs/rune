use crate::ast;
use crate::ast::Kind;
use crate::unit_builder::UnitBuilderError;
use crate::SourceId;
use runestick::{CompileMeta, Item, Span, Url};
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// A compile result.
pub type CompileResult<T, E = CompileError> = std::result::Result<T, E>;

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
    #[error("expected end of input, but encountered `{actual}`")]
    ExpectedEof {
        /// Span that caused the error.
        span: Span,
        /// Kind of the token encountered instead of end-of-file.
        actual: Kind,
    },
    /// Error raised when we expect a declaration.
    #[error("expected declaration `fn`, `mod`, `struct`, `enum`, or `use`. got `{actual}`.")]
    ExpectedItem {
        /// Span that caused the error.
        span: Span,
        /// Kind of the token encountered instead of a declaration.
        actual: Kind,
    },
    /// Expected use import but found something else.
    #[error("expected import component but found `{actual}`")]
    ExpectedItemUseImportComponent {
        /// The span of the component.
        span: Span,
        /// The actual token kind.
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
    /// Attempt to get a value for a synthetic identifier.
    #[error("tried to get bad synthetic identifier `{id}` for {kind}")]
    BadSyntheticId {
        /// The kind of id we tried to fetch.
        kind: &'static str,
        /// The slice we tried to read.
        span: Span,
        /// The identifier that was bad.
        id: usize,
    },
    /// Encountered a bad string escape sequence.
    #[error("bad escape sequence")]
    BadEscapeSequence {
        /// Span of the illegal escape sequence.
        span: Span,
    },
    /// Tried to resolve an illegal number literal.
    #[error("number literal not valid")]
    BadNumberLiteral {
        /// Span of the illegal number literal.
        span: Span,
    },
    /// Number out of bounds.
    #[error("number literal out of bounds `-9223372036854775808` to `9223372036854775807`")]
    BadNumberOutOfBounds {
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
    /// When we encounter an expression that cannot be used in a chained manner.
    #[error("unsupported field access")]
    UnsupportedFieldAccess {
        /// Span of the expression that can't be used in a chain.
        span: Span,
    },
    /// Trying to use a token that is not supported as a function argument.
    #[error("not supported as a function or closure argument")]
    ExpectedFunctionArgument {
        /// Where the argument is.
        span: Span,
    },
    /// Trying to use an expression as async when it's not supported.
    #[error("not supported as an async expression")]
    UnsupportedAsyncExpr {
        /// Where the expression is.
        span: Span,
    },
    /// Expected a macro delimiter.
    #[error("expected delimiter, `(`, `[`, or `{{`, but got `{actual}`")]
    ExpectedMacroDelimiter {
        /// Span of token we saw instead.
        span: Span,
        /// What we actually saw.
        actual: Kind,
    },
    /// Expected a macro close delimiter.
    #[error("expected close delimiter `{expected}`, but got `{actual}`")]
    ExpectedMacroCloseDelimiter {
        /// Span of token we saw instead.
        span: Span,
        /// The delimiter we expected.
        expected: Kind,
        /// The delimiter we saw.
        actual: Kind,
    },
    /// Expected a block semicolon which is needed for the kind of expression.
    #[error("expected expression to be terminated by a semicolon `;`")]
    ExpectedBlockSemiColon {
        /// Span where we expected the semicolon.
        span: Span,
        /// The following expression.
        followed_span: Span,
    },
}

impl ParseError {
    /// Get the span for the parse error.
    pub fn span(&self) -> Span {
        match *self {
            Self::UnexpectedEof { span, .. } => span,
            Self::ExpectedEof { span, .. } => span,
            Self::ExpectedItem { span, .. } => span,
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
            Self::ExpectedUnaryOperator { span, .. } => span,
            Self::PrecedenceGroupRequired { span, .. } => span,
            Self::BadSlice { span, .. } => span,
            Self::BadSyntheticId { span, .. } => span,
            Self::BadEscapeSequence { span, .. } => span,
            Self::BadNumberLiteral { span, .. } => span,
            Self::BadNumberOutOfBounds { span, .. } => span,
            Self::BadCharLiteral { span, .. } => span,
            Self::BadByteLiteral { span, .. } => span,
            Self::UnicodeEscapeNotSupported { span, .. } => span,
            Self::BadUnicodeEscape { span, .. } => span,
            Self::UnsupportedUnicodeByteEscape { span, .. } => span,
            Self::UnsupportedByteEscape { span, .. } => span,
            Self::BadByteEscape { span, .. } => span,
            Self::InvalidTemplateLiteral { span, .. } => span,
            Self::UnexpectedCloseBrace { span, .. } => span,
            Self::UnsupportedFieldAccess { span, .. } => span,
            Self::ExpectedFunctionArgument { span, .. } => span,
            Self::ExpectedItemUseImportComponent { span, .. } => span,
            Self::UnsupportedAsyncExpr { span, .. } => span,
            Self::ExpectedMacroDelimiter { span, .. } => span,
            Self::ExpectedMacroCloseDelimiter { span, .. } => span,
            Self::ExpectedBlockSemiColon { span, .. } => span,
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
    /// Trying to use an experimental feature which was not enabled.
    #[error("experimental feature: {msg}")]
    Experimental {
        /// The message of the variant.
        msg: &'static str,
        /// Where the experimental feature was used.
        span: Span,
    },
    /// Cannot find a file corresponding to a module.
    #[error("file not found, expected a module file like `{path}.rn`")]
    ModNotFound {
        /// Path where file failed to be loaded from.
        path: PathBuf,
        /// Span of the
        span: Span,
    },
    /// Failed to load file from the given path.
    #[error("failed to load `{path}`: {error}")]
    ModFileError {
        /// Path where file failed to be loaded from.
        path: PathBuf,
        /// Span of the
        span: Span,
        /// The underlying error.
        #[source]
        error: io::Error,
    },
    /// A module that has already been loaded.
    #[error("module `{item}` has already been loaded")]
    ModAlreadyLoaded {
        /// Base path of a module that has already been loaded.
        item: Item,
        /// Span of the
        span: Span,
        /// The existing location of the module.
        existing: (SourceId, Span),
    },
    /// Unit error from runestick encoding.
    #[error("unit construction error: {error}")]
    UnitBuilderError {
        /// Source error.
        #[from]
        error: UnitBuilderError,
    },
    /// Error for resolving values from source files.
    #[error("{error}")]
    ParseError {
        /// Source error.
        #[from]
        error: ParseError,
    },
    /// Error when trying to index a duplicate item.
    #[error("found conflicting item `{existing}`")]
    ItemConflict {
        /// Where the conflicting item was found.
        span: Span,
        /// The name of the conflicting item.
        existing: Item,
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
    /// Error missing a macro.
    #[error("missing macro `{item}`")]
    MissingMacro {
        /// The span where the macro was missing.
        span: Span,
        /// Name of the missing macro.
        item: Item,
    },
    /// Error while calling macro.
    #[error("error while calling macro: {error}")]
    CallMacroError {
        /// The span where the macro was called.
        span: Span,
        /// Source error.
        error: runestick::Error,
    },
    /// Error for missing local variables.
    #[error("missing variable `{name}`")]
    MissingLocal {
        /// Span where the error occured.
        span: Span,
        /// Name of the missing variable.
        name: String,
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
    #[error("missing module `{item}`")]
    MissingModule {
        /// The span of the missing module.
        span: Span,
        /// The name of the missing module.
        item: Item,
    },
    /// A specific label is missing.
    #[error("label not found in scope")]
    MissingLabel {
        /// The span of the missing label.
        span: Span,
    },
    /// Encountered an unsupported URL when loading a module.
    #[error("cannot load the url `{url}`")]
    UnsupportedLoadUrl {
        /// The span where the unsupported URL was encountered.
        span: Span,
        /// The URL that was unsupported.
        url: Url,
    },
    /// Unsupported wildcard component in use.
    #[error("wildcard support not supported in this position")]
    UnsupportedWildcard {
        /// Where the wildcard import is.
        span: Span,
    },
    /// Tried to use a meta as an async block for which it is not supported.
    #[error("`{meta}` is not a supported async block")]
    UnsupportedAsyncBlock {
        /// The span where we tried to use an async block.
        span: Span,
        /// The meta we tried to use as an async block.
        meta: CompileMeta,
    },
    /// Tried to declare an instance function on a type for which it is not
    /// supported.
    #[error("cannot declare instance functions for type `{meta}`")]
    UnsupportedInstanceFunction {
        /// The span where we tried to declare an instance function.
        span: Span,
        /// The meta we tried to declare an instance function for.
        meta: CompileMeta,
    },
    /// Tried to treat something as a value which is not supported.
    #[error("`{meta}` cannot be used as a value")]
    UnsupportedValue {
        /// The span of the error.
        span: Span,
        /// The meta we tried to treat as a value.
        meta: CompileMeta,
    },
    /// Tried to treat something as a type which is not supported.
    #[error("`{meta}` cannot be used as a type")]
    UnsupportedType {
        /// The span of the error.
        span: Span,
        /// The meta we tried to treat as a type.
        meta: CompileMeta,
    },
    /// `self` occured in an unsupported position.
    #[error("`self` not supported here")]
    UnsupportedSelf {
        /// Where it occured.
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
    /// When we encounter an expression that cannot be assigned to.
    #[error("unsupported binary expression")]
    UnsupportedBinaryExpr {
        /// The location of the expression.
        span: Span,
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
        meta: CompileMeta,
        /// The expected number of arguments.
        expected: usize,
        /// The actual number of arguments.
        actual: usize,
    },
    /// A meta item that is not supported in the given pattern position.
    #[error("`{meta}` is not supported in a pattern like this")]
    UnsupportedMetaPattern {
        /// The meta item we tried to use as a pattern.
        meta: CompileMeta,
        /// The span which the error occured.
        span: Span,
    },
    /// A meta item that is not supported in the given closure position.
    #[error("`{meta}` is not supported as a closure")]
    UnsupportedMetaClosure {
        /// The meta item we tried to use as a pattern.
        meta: CompileMeta,
        /// The span which the error occured.
        span: Span,
    },
    /// The pattern is not supported.
    #[error("item is not supported in a pattern")]
    UnsupportedPattern {
        /// Span where the error occured.
        span: Span,
    },
    /// The pattern is not supported as a binding.
    #[error("not a valid binding")]
    UnsupportedBinding {
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
    #[error("`{item}` is not a function")]
    MissingFunction {
        /// The span of the unsupported function call.
        span: Span,
        /// The item we're trying to call.
        item: Item,
    },
    /// Attempt to yield outside of a function or a closure.
    #[error("`yield` must be used in function or closure")]
    YieldOutsideFunction {
        /// The span of the unsupported yield.
        span: Span,
    },
    /// Attempt to await outside of a function or a closure.
    #[error("`await` must be used inside an async function or closure")]
    AwaitOutsideFunction {
        /// The span of the unsupported await.
        span: Span,
    },
    /// Attempt to declare a function which takes `self` outside of an `impl`
    /// block.
    #[error("instance function declared outside of `impl` block")]
    InstanceFunctionOutsideImpl {
        /// Where the function is declared.
        span: Span,
    },
    /// Import doesn't exist.
    #[error("import `{item}` (imported in prelude) does not exist")]
    MissingPreludeModule {
        /// The item that didn't exist.
        item: Item,
    },
    /// Trying to use an expression as async when it's not supported.
    #[error("not supported as an async expression")]
    UnsupportedAsyncExpr {
        /// Where the expression is.
        span: Span,
    },
    /// Trying to use a filesystem module from an in-memory soruce.
    #[error("cannot load external modules from in-memory sources")]
    UnsupportedFileMod {
        /// The span where the error happened.
        span: Span,
    },
    /// Trying to use a number as a tuple index for which it is not suported.
    #[error("unsupported tuple index `{number}`")]
    UnsupportedTupleIndex {
        /// The number that was an unsupported tuple index.
        number: ast::Number,
        /// Location of the unsupported tuple index.
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

    /// Construct an experimental error.
    ///
    /// This should be used when an experimental feature is used which hasn't
    /// been enabled.
    pub fn experimental(msg: &'static str, span: Span) -> Self {
        Self::Experimental { msg, span }
    }
}

impl CompileError {
    /// Get the span for the error.
    pub fn span(&self) -> Span {
        match *self {
            Self::UnitBuilderError { .. } => Span::default(),
            Self::Internal { span, .. } => span,
            Self::Experimental { span, .. } => span,
            Self::ModNotFound { span, .. } => span,
            Self::ModFileError { span, .. } => span,
            Self::ModAlreadyLoaded { span, .. } => span,
            Self::ParseError { error, .. } => error.span(),
            Self::ItemConflict { span, .. } => span,
            Self::VariableConflict { span, .. } => span,
            Self::MissingMacro { span, .. } => span,
            Self::CallMacroError { span, .. } => span,
            Self::MissingLocal { span, .. } => span,
            Self::MissingType { span, .. } => span,
            Self::MissingModule { span, .. } => span,
            Self::MissingLabel { span, .. } => span,
            Self::UnsupportedLoadUrl { span, .. } => span,
            Self::UnsupportedWildcard { span, .. } => span,
            Self::UnsupportedRef { span, .. } => span,
            Self::UnsupportedAwait { span, .. } => span,
            Self::UnsupportedAsyncBlock { span, .. } => span,
            Self::UnsupportedInstanceFunction { span, .. } => span,
            Self::UnsupportedValue { span, .. } => span,
            Self::UnsupportedType { span, .. } => span,
            Self::UnsupportedSelf { span, .. } => span,
            Self::UnsupportedUnaryOp { span, .. } => span,
            Self::UnsupportedBinaryOp { span, .. } => span,
            Self::UnsupportedLitObject { span, .. } => span,
            Self::UnsupportedAssignExpr { span, .. } => span,
            Self::UnsupportedBinaryExpr { span, .. } => span,
            Self::UnsupportedSelectPattern { span, .. } => span,
            Self::UnsupportedFieldAccess { span, .. } => span,
            Self::UnsupportedArgumentCount { span, .. } => span,
            Self::UnsupportedMetaPattern { span, .. } => span,
            Self::UnsupportedMetaClosure { span, .. } => span,
            Self::UnsupportedPattern { span, .. } => span,
            Self::UnsupportedBinding { span, .. } => span,
            Self::BreakOutsideOfLoop { span, .. } => span,
            Self::ReturnLocalReferences { span, .. } => span,
            Self::MatchFloatInPattern { span, .. } => span,
            Self::DuplicateObjectKey { span, .. } => span,
            Self::LitObjectMissingField { span, .. } => span,
            Self::LitObjectNotField { span, .. } => span,
            Self::MissingFunction { span, .. } => span,
            Self::YieldOutsideFunction { span, .. } => span,
            Self::AwaitOutsideFunction { span, .. } => span,
            Self::InstanceFunctionOutsideImpl { span, .. } => span,
            Self::MissingPreludeModule { .. } => Span::empty(),
            Self::UnsupportedAsyncExpr { span, .. } => span,
            Self::UnsupportedFileMod { span, .. } => span,
            Self::UnsupportedTupleIndex { span, .. } => span,
        }
    }
}
