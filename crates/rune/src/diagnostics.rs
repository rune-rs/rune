//! Diagnostics module for Rune.
//!
//! Diagnostics collects information about a source program in order to provide
//! good human-readable diagnostics like errors, warnings, and hints.
//!
//! In order to collect diagnostics during compilation you must register a
//! [Diagnostics] object with
//! [Build::with_diagnostics][crate::Build::with_diagnostics].
//!
//! ```
//! use rune::termcolor::{ColorChoice, StandardStream};
//! use rune::{Sources, Diagnostics};
//!
//! let mut sources = Sources::new();
//!
//! let mut diagnostics = Diagnostics::new();
//!
//! let result = rune::prepare(&mut sources)
//!     .with_diagnostics(&mut diagnostics)
//!     .build();
//!
//! // We delay unwrapping result into a unit so that we can print diagnostics
//! // even if they relate to the compilation failing.
//! if !diagnostics.is_empty() {
//!     let mut writer = StandardStream::stderr(ColorChoice::Always);
//!     diagnostics.emit(&mut writer, &sources)?;
//! }
//!
//! let unit = result?;
//! # Ok::<_, rune::Error>(())
//! ```

use crate::no_std::prelude::*;

use crate::ast::Span;
use crate::SourceId;

mod fatal;
pub use self::fatal::{FatalDiagnostic, FatalDiagnosticKind};

mod warning;
pub use self::warning::{WarningDiagnostic, WarningDiagnosticKind};

cfg_emit! {
    mod emit;
    #[doc(inline)]
    pub use self::emit::EmitError;
}

/// A single diagnostic.
#[derive(Debug)]
#[non_exhaustive]
pub enum Diagnostic {
    /// A fatal diagnostic.
    Fatal(FatalDiagnostic),
    /// A warning diagnostic.
    Warning(WarningDiagnostic),
}

/// The diagnostics mode to use.
#[derive(Debug, Clone, Copy)]
enum Mode {
    /// Collect all forms of diagnostics.
    All,
    /// Collect errors.
    WithoutWarnings,
}

impl Mode {
    /// If warnings are enabled.
    fn warnings(self) -> bool {
        matches!(self, Self::All)
    }
}

/// Structure to collect compilation diagnostics.
///
/// If the project is compiled with the `emit` feature, you can make use of
/// [Diagnostics::emit].
///
/// # Examples
///
/// ```,no_run
/// use rune::{Sources, Diagnostics};
/// use rune::termcolor::{StandardStream, ColorChoice};
///
/// let mut sources = Sources::new();
/// let mut diagnostics = Diagnostics::new();
///
/// // use sources and diagnostics to compile a project.
///
/// if !diagnostics.is_empty() {
///     let mut writer = StandardStream::stderr(ColorChoice::Always);
///     diagnostics.emit(&mut writer, &sources)?;
/// }
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Debug)]
pub struct Diagnostics {
    diagnostics: Vec<Diagnostic>,
    /// If warnings are collected or not.
    mode: Mode,
    /// Indicates if diagnostics indicates errors.
    has_error: bool,
    /// Indicates if diagnostics contains warnings.
    has_warning: bool,
}

impl Diagnostics {
    fn with_mode(mode: Mode) -> Self {
        Self {
            diagnostics: Vec::new(),
            mode,
            has_error: false,
            has_warning: false,
        }
    }

    /// Construct a new, empty collection of compilation warnings that is
    /// disabled, i.e. any warnings added to it will be ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Diagnostics, SourceId};
    /// use rune::ast::Span;
    ///
    /// let mut diagnostics = Diagnostics::without_warnings();
    /// assert!(diagnostics.is_empty());
    ///
    /// // emit warnings
    ///
    /// assert!(diagnostics.is_empty());
    /// let warning = diagnostics.into_diagnostics().into_iter().next();
    /// assert!(matches!(warning, None));
    /// ```
    pub fn without_warnings() -> Self {
        Self::with_mode(Mode::WithoutWarnings)
    }

    /// Construct a new, empty collection of compilation warnings.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Diagnostics, SourceId};
    /// use rune::ast::Span;
    /// use rune::diagnostics::Diagnostic;
    ///
    /// let mut diagnostics = Diagnostics::new();
    /// assert!(diagnostics.is_empty());
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
        self.has_error
    }

    /// Check if diagnostics has any warnings reported.
    pub fn has_warning(&self) -> bool {
        self.has_warning
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
        self.error(source_id, FatalDiagnosticKind::Internal(message));
    }

    /// Indicate that a value is produced but never used.
    pub(crate) fn not_used(&mut self, source_id: SourceId, span: Span, context: Option<Span>) {
        self.warning(source_id, WarningDiagnosticKind::NotUsed { span, context });
    }

    /// Indicate that a binding pattern might panic.
    ///
    /// Like `let (a, b) = value`.
    pub(crate) fn let_pattern_might_panic(
        &mut self,
        source_id: SourceId,
        span: Span,
        context: Option<Span>,
    ) {
        self.warning(
            source_id,
            WarningDiagnosticKind::LetPatternMightPanic { span, context },
        );
    }

    /// Indicate that we encountered a template string without any expansion
    /// groups.
    ///
    /// Like `` `Hello` ``.
    pub(crate) fn template_without_expansions(
        &mut self,
        source_id: SourceId,
        span: Span,
        context: Option<Span>,
    ) {
        self.warning(
            source_id,
            WarningDiagnosticKind::TemplateWithoutExpansions { span, context },
        );
    }

    /// Add a warning indicating that the parameters of an empty tuple can be
    /// removed when creating it.
    ///
    /// Like `None()`.
    pub(crate) fn remove_tuple_call_parens(
        &mut self,
        source_id: SourceId,
        span: Span,
        variant: Span,
        context: Option<Span>,
    ) {
        self.warning(
            source_id,
            WarningDiagnosticKind::RemoveTupleCallParams {
                span,
                variant,
                context,
            },
        );
    }

    /// Add a warning about an unecessary semi-colon.
    pub(crate) fn unnecessary_semi_colon(&mut self, source_id: SourceId, span: Span) {
        self.warning(
            source_id,
            WarningDiagnosticKind::UnnecessarySemiColon { span },
        );
    }

    /// Push a warning to the collection of diagnostics.
    pub(crate) fn warning<T>(&mut self, source_id: SourceId, kind: T)
    where
        WarningDiagnosticKind: From<T>,
    {
        if !self.mode.warnings() {
            return;
        }

        self.diagnostics
            .push(Diagnostic::Warning(WarningDiagnostic {
                source_id,
                kind: kind.into(),
            }));

        self.has_warning = true;
    }

    /// Report an error.
    pub(crate) fn error<T>(&mut self, source_id: SourceId, kind: T)
    where
        FatalDiagnosticKind: From<T>,
    {
        self.diagnostics.push(Diagnostic::Fatal(FatalDiagnostic {
            source_id,
            kind: Box::new(kind.into()),
        }));

        self.has_error = true;
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::with_mode(Mode::All)
    }
}
