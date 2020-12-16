use crate::compiling::LinkerError;
use crate::{BuildError, CompileError, ParseError, QueryError, Spanned};
use runestick::{SourceId, Span};
use std::error;
use std::fmt;
use thiserror::Error;

/// An error raised when using one of the `load_*` functions.
#[derive(Debug)]
pub struct Error {
    /// Last error in chain of reported errors.
    last: Option<usize>,
    /// The source id of the error.
    source_id: SourceId,
    /// The kind of the load error.
    kind: Box<ErrorKind>,
}

impl Error {
    /// The source id where the error originates from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// The kind of the load error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Convert into the kind of the load error.
    pub fn into_kind(self) -> ErrorKind {
        *self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("parse error")]
    ParseError(
        #[from]
        #[source]
        ParseError,
    ),
    #[error("compile error")]
    CompileError(
        #[from]
        #[source]
        CompileError,
    ),
    #[error("query error")]
    QueryError(
        #[from]
        #[source]
        QueryError,
    ),
    #[error("linker error")]
    LinkError(
        #[from]
        #[source]
        LinkerError,
    ),
    #[error("builder error: {0}")]
    BuildError(
        #[from]
        #[source]
        BuildError,
    ),
    /// An internal error.
    #[error("internal error: {0}")]
    Internal(&'static str),
}

/// Compilation warning.
#[derive(Debug, Clone, Copy)]
pub struct Warning {
    /// The last warning reported in the chain.
    last: Option<usize>,
    /// The id of the source where the warning happened.
    source_id: SourceId,
    /// The kind of the warning.
    kind: WarningKind,
}

impl Warning {
    /// The source id where the warning originates from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// The kind of the warning.
    pub fn kind(&self) -> &WarningKind {
        &self.kind
    }

    /// Convert into the kind of the warning.
    pub fn into_kind(self) -> WarningKind {
        self.kind
    }

    /// Get the span of the warning.
    pub fn span(&self) -> Span {
        match &self.kind {
            WarningKind::NotUsed { span, .. } => *span,
            WarningKind::LetPatternMightPanic { span, .. } => *span,
            WarningKind::TemplateWithoutExpansions { span, .. } => *span,
            WarningKind::RemoveTupleCallParams { span, .. } => *span,
            WarningKind::UnecessarySemiColon { span, .. } => *span,
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl error::Error for Warning {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

/// Compilation warning kind.
#[derive(Debug, Clone, Copy, Error)]
pub enum WarningKind {
    /// Item identified by the span is not used.
    #[error("not used")]
    NotUsed {
        /// The span that is not used.
        span: Span,
        /// The context in which the value was not used.
        context: Option<Span>,
    },
    /// Warning that an unconditional let pattern will panic if it doesn't
    /// match.
    #[error("pattern might panic")]
    LetPatternMightPanic {
        /// The span of the pattern.
        span: Span,
        /// The context in which it is used.
        context: Option<Span>,
    },
    /// Encountered a template string without an expansion.
    #[error("using a template string without expansions, like `Hello World`")]
    TemplateWithoutExpansions {
        /// Span that caused the error.
        span: Span,
        /// The context in which it is used.
        context: Option<Span>,
    },
    /// Suggestion that call parameters could be removed.
    #[error("call paramters are not needed here")]
    RemoveTupleCallParams {
        /// The span of the call.
        span: Span,
        /// The span of the variant being built.
        variant: Span,
        /// The context in which it is used.
        context: Option<Span>,
    },
    /// An unecessary semi-colon is used.
    #[error("unnecessary semicolon")]
    UnecessarySemiColon {
        /// Span where the semi-colon is.
        span: Span,
    },
}

/// A single diagnostic.
#[derive(Debug)]
pub enum Diagnostic {
    /// An error diagnostic.
    Error(Error),
    /// A warning diagnostic.
    Warning(Warning),
}

/// Compilation warnings.
#[derive(Debug)]
pub struct Diagnostics {
    diagnostics: Vec<Diagnostic>,
    /// If warnings are collected or not.
    warnings: bool,
    /// First error in chain.
    last_error: Option<usize>,
    /// First warning in chain.
    last_warning: Option<usize>,
}

impl Diagnostics {
    /// Construct a new, empty collection of compilation warnings that is
    /// disabled, i.e. any warnings added to it will be ignored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::{Diagnostic, Diagnostics};
    /// use runestick::Span;
    ///
    /// let mut diagnostics = Diagnostics::without_warnings();
    /// assert!(diagnostics.is_empty());
    ///
    /// diagnostics.not_used(0, Span::empty(), None);
    ///
    /// assert!(diagnostics.is_empty());
    /// let warning = diagnostics.into_diagnostics().into_iter().next();
    /// assert!(matches!(warning, None));
    /// ```
    pub fn without_warnings() -> Self {
        Self {
            diagnostics: Vec::new(),
            warnings: false,
            last_error: None,
            last_warning: None,
        }
    }

    /// Construct a new, empty collection of compilation warnings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::{Diagnostic, Diagnostics, Warning, WarningKind};
    /// use runestick::Span;
    ///
    /// let mut diagnostics = Diagnostics::new();
    /// assert!(diagnostics.is_empty());
    ///
    /// diagnostics.not_used(0, Span::empty(), None);
    ///
    /// assert!(!diagnostics.is_empty());
    ///
    /// assert!(matches! {
    ///     diagnostics.into_diagnostics().into_iter().next(),
    ///     Some(Diagnostic::Warning(..))
    /// });
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Indicate if there is any diagnostics.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Check if diagnostics has any errors reported.
    pub fn has_error(&self) -> bool {
        self.last_error.is_some()
    }

    /// Check if diagnostics has any warnings reported.
    pub fn has_warning(&self) -> bool {
        self.last_warning.is_some()
    }

    /// Access underlying diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Convert into underlying diagnostics.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Report an error.
    pub fn error<E>(&mut self, source_id: SourceId, kind: E)
    where
        ErrorKind: From<E>,
    {
        let current = Some(self.diagnostics.len());

        self.diagnostics.push(Diagnostic::Error(Error {
            last: self.last_error,
            source_id,
            kind: Box::new(kind.into()),
        }));

        self.last_error = current;
    }

    /// Report an internal error.
    ///
    /// This should be used for programming invariants of the compiler which are
    /// broken for some reason.
    pub(crate) fn internal(&mut self, source_id: SourceId, message: &'static str) {
        self.error(source_id, ErrorKind::Internal(message));
    }

    /// Indicate that a value is produced but never used.
    pub fn not_used<S>(&mut self, source_id: usize, spanned: S, context: Option<Span>)
    where
        S: Spanned,
    {
        self.warning(
            source_id,
            WarningKind::NotUsed {
                span: spanned.span(),
                context,
            },
        );
    }

    /// Indicate that a binding pattern might panic.
    ///
    /// Like `let (a, b) = value`.
    pub fn let_pattern_might_panic(&mut self, source_id: usize, span: Span, context: Option<Span>) {
        self.warning(
            source_id,
            WarningKind::LetPatternMightPanic { span, context },
        );
    }

    /// Indicate that we encountered a template string without any expansion
    /// groups.
    ///
    /// Like `` `Hello` ``.
    pub fn template_without_expansions(
        &mut self,
        source_id: usize,
        span: Span,
        context: Option<Span>,
    ) {
        self.warning(
            source_id,
            WarningKind::TemplateWithoutExpansions { span, context },
        );
    }

    /// Add a warning indicating that the parameters of an empty tuple can be
    /// removed when creating it.
    ///
    /// Like `None()`.
    pub fn remove_tuple_call_parens(
        &mut self,
        source_id: usize,
        span: Span,
        variant: Span,
        context: Option<Span>,
    ) {
        self.warning(
            source_id,
            WarningKind::RemoveTupleCallParams {
                span,
                variant,
                context,
            },
        );
    }

    /// Add a warning about an unecessary semi-colon.
    pub fn uneccessary_semi_colon(&mut self, source_id: usize, span: Span) {
        self.warning(source_id, WarningKind::UnecessarySemiColon { span });
    }

    /// Push a warning to the collection of diagnostics.
    fn warning(&mut self, source_id: SourceId, kind: WarningKind) {
        if !self.warnings {
            return;
        }

        let current = Some(self.diagnostics.len());
        self.diagnostics.push(Diagnostic::Warning(Warning {
            last: self.last_warning,
            source_id,
            kind,
        }));
        self.last_warning = current;
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            diagnostics: Vec::new(),
            warnings: true,
            last_error: None,
            last_warning: None,
        }
    }
}
