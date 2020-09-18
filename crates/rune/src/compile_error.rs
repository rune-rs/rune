use crate::ast;
use crate::unit_builder::UnitBuilderError;
use crate::{ParseError, ParseErrorKind, Spanned};
use runestick::{CompileMeta, Item, SourceId, Span};
use std::error;
use std::fmt;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// A compile result.
pub type CompileResult<T, E = CompileError> = std::result::Result<T, E>;

/// An error raised during compiling.
#[derive(Debug)]
pub struct CompileError {
    span: Span,
    kind: CompileErrorKind,
}

impl CompileError {
    /// Construct a new compile error.
    pub fn new<S, E>(spanned: S, err: E) -> Self
    where
        S: Spanned,
        CompileErrorKind: From<E>,
    {
        Self {
            span: spanned.span(),
            kind: CompileErrorKind::from(err),
        }
    }

    /// Get the kind of the cmopile error.
    pub fn kind(&self) -> &CompileErrorKind {
        &self.kind
    }

    /// Convert into the kind of the compile error.
    pub fn into_kind(self) -> CompileErrorKind {
        self.kind
    }

    /// Construct an internal error.
    ///
    /// This should be used for programming invariants of the encoder which are
    /// broken for some reason.
    pub fn internal(span: Span, msg: &'static str) -> Self {
        CompileError::new(span, CompileErrorKind::Internal { msg })
    }

    /// Construct an experimental error.
    ///
    /// This should be used when an experimental feature is used which hasn't
    /// been enabled.
    pub fn experimental(span: Span, msg: &'static str) -> Self {
        Self::new(span, CompileErrorKind::Experimental { msg })
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl error::Error for CompileError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

impl Spanned for CompileError {
    /// Get the span for the parse error.
    fn span(&self) -> Span {
        self.span
    }
}

impl From<ParseError> for CompileError {
    fn from(error: ParseError) -> Self {
        CompileError {
            span: error.span(),
            kind: CompileErrorKind::ParseError {
                error: error.into_kind(),
            },
        }
    }
}

impl From<UnitBuilderError> for CompileError {
    fn from(error: UnitBuilderError) -> Self {
        CompileError {
            span: Span::empty(),
            kind: CompileErrorKind::UnitBuilderError { error },
        }
    }
}

/// Error when encoding AST.
#[derive(Debug, Error)]
pub enum CompileErrorKind {
    /// An internal encoder invariant was broken.
    #[error("internal compiler error: {msg}")]
    Internal {
        /// The message of the variant.
        msg: &'static str,
    },
    /// Trying to use an experimental feature which was not enabled.
    #[error("experimental feature: {msg}")]
    Experimental {
        /// The message of the variant.
        msg: &'static str,
    },
    /// Cannot find a file corresponding to a module.
    #[error("file not found, expected a module file like `{path}.rn`")]
    ModNotFound {
        /// Path where file failed to be loaded from.
        path: PathBuf,
    },
    /// Failed to load file from the given path.
    #[error("failed to load `{path}`: {error}")]
    ModFileError {
        /// Path where file failed to be loaded from.
        path: PathBuf,
        /// The underlying error.
        #[source]
        error: io::Error,
    },
    /// A module that has already been loaded.
    #[error("module `{item}` has already been loaded")]
    ModAlreadyLoaded {
        /// Base path of a module that has already been loaded.
        item: Item,
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
        error: ParseErrorKind,
    },
    /// Error when trying to index a duplicate item.
    #[error("found conflicting item `{existing}`")]
    ItemConflict {
        /// The name of the conflicting item.
        existing: Item,
    },
    /// Error for variable conflicts.
    #[error("variable `{name}` conflicts")]
    VariableConflict {
        /// Name of the conflicting variable.
        name: String,
        /// The span where the variable was already present.
        existing_span: Span,
    },
    /// Error missing a macro.
    #[error("missing macro `{item}`")]
    MissingMacro {
        /// Name of the missing macro.
        item: Item,
    },
    /// Error while calling macro.
    #[error("error while calling macro: {error}")]
    CallMacroError {
        /// Source error.
        error: runestick::Error,
    },
    /// Error for missing local variables.
    #[error("no local variable `{name}`")]
    MissingLocal {
        /// Name of the missing variable.
        name: String,
    },
    /// Error for missing types.
    #[error("no such type `{item}`")]
    MissingType {
        /// Name of the missing type.
        item: Item,
    },
    /// Tried to use a module that was missing.
    #[error("missing module `{item}`")]
    MissingModule {
        /// The name of the missing module.
        item: Item,
    },
    /// A specific label is missing.
    #[error("label not found in scope")]
    MissingLabel,
    /// Tried to load module in a source where it wasn't supported.
    #[error("cannot load modules using a source without an associated URL")]
    UnsupportedModuleSource,
    /// Encountered an unsupported URL when loading a module.
    #[error("cannot load modules relative to `{root}`")]
    UnsupportedModuleRoot {
        /// The Path that was unsupported.
        root: PathBuf,
    },
    /// Encountered an unsupported Item when loading a module.
    #[error("cannot load module for `{item}`")]
    UnsupportedModuleItem {
        /// The item that cannot be used as a module.
        item: Item,
    },
    /// Unsupported wildcard component in use.
    #[error("wildcard support not supported in this position")]
    UnsupportedWildcard,
    /// Tried to use a meta as an async block for which it is not supported.
    #[error("`{meta}` is not a supported async block")]
    UnsupportedAsyncBlock {
        /// The meta we tried to use as an async block.
        meta: CompileMeta,
    },
    /// Tried to declare an instance function on a type for which it is not
    /// supported.
    #[error("cannot declare instance functions for type `{meta}`")]
    UnsupportedInstanceFunction {
        /// The meta we tried to declare an instance function for.
        meta: CompileMeta,
    },
    /// Tried to treat something as a value which is not supported.
    #[error("`{meta}` cannot be used as a value")]
    UnsupportedValue {
        /// The meta we tried to treat as a value.
        meta: CompileMeta,
    },
    /// Tried to treat something as a type which is not supported.
    #[error("`{meta}` cannot be used as a type")]
    UnsupportedType {
        /// The meta we tried to treat as a type.
        meta: CompileMeta,
    },
    /// `self` occured in an unsupported position.
    #[error("`self` not supported here")]
    UnsupportedSelf,
    /// Encountered a unary operator we can't encode.
    #[error("unsupported unary operator `{op}`")]
    UnsupportedUnaryOp {
        /// The operator.
        op: ast::UnaryOp,
    },
    /// Encountered a binary operator we can't encode.
    #[error("unsupported binary operator `{op}`")]
    UnsupportedBinaryOp {
        /// The operator.
        op: ast::BinOp,
    },
    /// Cannot crate object literal of the given type.
    #[error("type `{item}` is not an object")]
    UnsupportedLitObject {
        /// The path to the unsupported object.
        item: Item,
    },
    /// Key is not present in the given type literal.
    #[error("missing field `{field}` in declaration of `{item}`")]
    LitObjectMissingField {
        /// They key that didn't exist.
        field: String,
        /// The related item.
        item: Item,
    },
    /// Key is not present in the given type literal.
    #[error("`{field}` is not a field in `{item}`")]
    LitObjectNotField {
        /// They key that is not a field.
        field: String,
        /// The related item.
        item: Item,
    },
    /// When we encounter an expression that cannot be assigned to.
    #[error("cannot assign to expression")]
    UnsupportedAssignExpr,
    /// When we encounter an expression that cannot be assigned to.
    #[error("unsupported binary expression")]
    UnsupportedBinaryExpr,
    /// When we encounter an expression that doesn't have a stack location and
    /// can't be referenced.
    #[error("cannot take reference of expression")]
    UnsupportedRef,
    /// Using a pattern that is not supported in a select.
    #[error("unsupported select pattern")]
    UnsupportedSelectPattern,
    /// Unsupported field access.
    #[error("unsupported field access")]
    UnsupportedFieldAccess,
    /// A meta item that is not supported in the given pattern position.
    #[error("wrong number of arguments, expected `{expected}` but got `{actual}`")]
    UnsupportedArgumentCount {
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
    },
    /// A meta item that is not supported in the given closure position.
    #[error("`{meta}` is not supported as a closure")]
    UnsupportedMetaClosure {
        /// The meta item we tried to use as a pattern.
        meta: CompileMeta,
    },
    /// The pattern is not supported.
    #[error("item is not supported in a pattern")]
    UnsupportedPattern,
    /// The pattern is not supported as a binding.
    #[error("not a valid binding")]
    UnsupportedBinding,
    /// Error raised when trying to use a break outside of a loop.
    #[error("break expressions cannot be used as a value")]
    BreakOutsideOfLoop,
    /// Attempting to use a float in a match pattern.
    #[error("floating point numbers cannot be used in patterns")]
    MatchFloatInPattern,
    /// Attempting to create an object with a duplicate object key.
    #[error("duplicate key in literal object")]
    DuplicateObjectKey {
        /// Where the object key exists previously.
        existing: Span,
        /// The object being defined.
        object: Span,
    },
    /// Attempt to call something that is not a function.
    #[error("`{item}` is not a function")]
    MissingFunction {
        /// The item we're trying to call.
        item: Item,
    },
    /// Attempt to yield outside of a function or a closure.
    #[error("`yield` must be used in function or closure")]
    YieldOutsideFunction,
    /// Attempt to await outside of a function or a closure.
    #[error("`await` must be used inside an async function or closure")]
    AwaitOutsideFunction,
    /// Attempt to declare a function which takes `self` outside of an `impl`
    /// block.
    #[error("instance function declared outside of `impl` block")]
    InstanceFunctionOutsideImpl,
    /// Import doesn't exist.
    #[error("import `{item}` (imported in prelude) does not exist")]
    MissingPreludeModule {
        /// The item that didn't exist.
        item: Item,
    },
    /// Trying to use a number as a tuple index for which it is not suported.
    #[error("unsupported tuple index `{number}`")]
    UnsupportedTupleIndex {
        /// The number that was an unsupported tuple index.
        number: ast::Number,
    },
}
