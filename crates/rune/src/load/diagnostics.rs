use crate::load::{Error, Warning, WarningKind};
use crate::Spanned;
use runestick::Span;

/// Compilation warnings.
#[derive(Debug)]
pub struct Diagnostics {
    errors: Vec<Error>,
    warnings: Option<Vec<Warning>>,
}

impl Diagnostics {
    /// Construct a new, empty collection of compilation warnings that is
    /// disabled, i.e. any warnings added to it will be ignored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::Diagnostics;
    /// use runestick::Span;
    ///
    /// let mut diagnostics = Diagnostics::without_warnings();
    /// assert!(diagnostics.is_empty());
    ///
    /// diagnostics.not_used(0, Span::empty(), None);
    ///
    /// assert!(diagnostics.is_empty());
    /// let warning = diagnostics.into_warnings().into_iter().next();
    /// assert!(matches!(warning, None));
    /// ```
    pub fn without_warnings() -> Self {
        Self {
            errors: Vec::new(),
            warnings: None,
        }
    }

    /// Construct a new, empty collection of compilation warnings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::{Diagnostics, Warning, WarningKind};
    /// use runestick::Span;
    ///
    /// let mut diagnostics = Diagnostics::new();
    /// assert!(diagnostics.is_empty());
    ///
    /// diagnostics.not_used(0, Span::empty(), None);
    ///
    /// assert!(!diagnostics.is_empty());
    /// let warning = diagnostics.into_warnings().into_iter().next();
    /// assert!(matches!(warning, Some(Warning { source_id: 0, kind: WarningKind::NotUsed { .. } })));
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Indicate if there is any diagnostics.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.warnings.as_ref().map(Vec::is_empty).unwrap_or(true)
    }

    /// Access underlying warnings.
    pub fn warnings(&self) -> &[Warning] {
        self.warnings.as_deref().unwrap_or_default()
    }

    /// Convert into underlying warnings.
    pub fn into_warnings(self) -> Vec<Warning> {
        self.warnings.unwrap_or_default()
    }

    /// Access underlying errors.
    pub fn errors(&self) -> &[Error] {
        self.errors.as_slice()
    }

    /// Convert into underlying errors.
    pub fn into_errors(self) -> Vec<Error> {
        self.errors
    }

    /// Push an error to the collection.
    pub fn error(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// Indicate that a value is produced but never used.
    pub fn not_used<S>(&mut self, source_id: usize, spanned: S, context: Option<Span>)
    where
        S: Spanned,
    {
        if let Some(w) = &mut self.warnings {
            w.push(Warning {
                source_id,
                kind: WarningKind::NotUsed {
                    span: spanned.span(),
                    context,
                },
            });
        }
    }

    /// Indicate that a binding pattern might panic.
    ///
    /// Like `let (a, b) = value`.
    pub fn let_pattern_might_panic(&mut self, source_id: usize, span: Span, context: Option<Span>) {
        if let Some(w) = &mut self.warnings {
            w.push(Warning {
                source_id,
                kind: WarningKind::LetPatternMightPanic { span, context },
            });
        }
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
        if let Some(w) = &mut self.warnings {
            w.push(Warning {
                source_id,
                kind: WarningKind::TemplateWithoutExpansions { span, context },
            });
        }
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
        if let Some(w) = &mut self.warnings {
            w.push(Warning {
                source_id,
                kind: WarningKind::RemoveTupleCallParams {
                    span,
                    variant,
                    context,
                },
            });
        }
    }

    /// Add a warning about an unecessary semi-colon.
    pub fn uneccessary_semi_colon(&mut self, source_id: usize, span: Span) {
        if let Some(w) = &mut self.warnings {
            w.push(Warning {
                source_id,
                kind: WarningKind::UnecessarySemiColon { span },
            });
        }
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Some(Vec::new()),
        }
    }
}
