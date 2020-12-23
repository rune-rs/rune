use runestick::{SourceId, Span};

mod error;
mod warning;

pub use self::error::{Error, ErrorKind};
pub use self::warning::{Warning, WarningKind};

/// A single diagnostic.
#[derive(Debug)]
pub enum Diagnostic {
    /// An error diagnostic.
    Error(Error),
    /// A warning diagnostic.
    Warning(Warning),
}

/// The diagnostics mode to use.
#[derive(Debug, Clone, Copy)]
enum DiagnosticsMode {
    /// Collect all forms of diagnostics.
    All,
    /// Collect errors.
    WithoutWarnings,
}

impl DiagnosticsMode {
    /// If warnings are enabled.
    fn warnings(self) -> bool {
        matches!(self, Self::All)
    }
}

/// Structure to collect compilation diagnostics.
///
/// If the project is compiled with the `diagnostics` feature, you can make use
/// of the `EmitDiagnostics` trait to emit human-readable diagnostics.
///
/// # Examples
///
/// ```rust,no_run
/// use rune::{Sources, Diagnostics, EmitDiagnostics};
/// use rune::termcolor::{StandardStream, ColorChoice};
///
/// # fn main() -> runestick::Result<()> {
/// let mut sources = Sources::new();
/// let mut diagnostics = Diagnostics::new();
///
/// // use sources and diagnostics to compile a project.
///
/// if !diagnostics.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     diagnostics.emit_diagnostics(&mut writer, &sources)?;
/// }
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct Diagnostics {
    diagnostics: Vec<Diagnostic>,
    /// If warnings are collected or not.
    mode: DiagnosticsMode,
    /// First error in chain.
    last_error: Option<usize>,
    /// First warning in chain.
    last_warning: Option<usize>,
}

impl Diagnostics {
    fn with_mode(mode: DiagnosticsMode) -> Self {
        Self {
            diagnostics: Vec::new(),
            mode,
            last_error: None,
            last_warning: None,
        }
    }

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
        Self::with_mode(DiagnosticsMode::WithoutWarnings)
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

    /// Report an internal error.
    ///
    /// This should be used for programming invariants of the compiler which are
    /// broken for some reason.
    pub(crate) fn internal(&mut self, source_id: SourceId, message: &'static str) {
        self.error(source_id, ErrorKind::Internal(message));
    }

    /// Indicate that a value is produced but never used.
    pub fn not_used(&mut self, source_id: usize, span: Span, context: Option<Span>) {
        self.warning(source_id, WarningKind::NotUsed { span, context });
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
    pub fn warning<T>(&mut self, source_id: SourceId, kind: T)
    where
        WarningKind: From<T>,
    {
        if !self.mode.warnings() {
            return;
        }

        let current = Some(self.diagnostics.len());

        self.diagnostics.push(Diagnostic::Warning(Warning {
            last: self.last_warning,
            source_id,
            kind: kind.into(),
        }));

        self.last_warning = current;
    }

    /// Report an error.
    pub fn error<T>(&mut self, source_id: SourceId, kind: T)
    where
        ErrorKind: From<T>,
    {
        let current = Some(self.diagnostics.len());

        self.diagnostics.push(Diagnostic::Error(Error {
            last: self.last_error,
            source_id,
            kind: Box::new(kind.into()),
        }));

        self.last_error = current;
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::with_mode(DiagnosticsMode::All)
    }
}
