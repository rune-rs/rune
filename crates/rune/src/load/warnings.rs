use crate::load::{Warning, WarningKind};
use crate::Spanned;
use runestick::Span;

/// Compilation warnings.
#[derive(Debug, Clone, Default)]
pub struct Warnings {
    warnings: Option<Vec<Warning>>,
}

impl Warnings {
    /// Construct a new, empty collection of compilation warnings that is
    /// disabled, i.e. any warnings added to it will be ignored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::Warnings;
    /// use runestick::Span;
    ///
    /// let mut warnings = Warnings::disabled();
    /// assert!(warnings.is_empty());
    /// warnings.not_used(0, Span::empty(), None);
    /// ```
    pub fn disabled() -> Self {
        Self { warnings: None }
    }

    /// Construct a new, empty collection of compilation warnings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::{Warnings, Warning, WarningKind};
    /// use runestick::Span;
    ///
    /// let mut warnings = Warnings::new();
    /// assert!(warnings.is_empty());
    /// warnings.not_used(0, Span::empty(), None);
    /// assert!(!warnings.is_empty());
    ///
    /// assert!(matches!(warnings.iter().next(), Some(Warning { source_id: 0, kind: WarningKind::NotUsed { .. } })));
    /// ```
    pub fn new() -> Self {
        Self {
            warnings: Some(Vec::new()),
        }
    }

    /// Indicate if there are warnings or not.
    pub fn is_empty(&self) -> bool {
        self.warnings.as_ref().map(Vec::is_empty).unwrap_or(true)
    }

    /// Get an iterator over all the warnings.
    pub fn iter(&self) -> impl Iterator<Item = &'_ Warning> {
        self.into_iter()
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

impl<'a> IntoIterator for &'a Warnings {
    type IntoIter = std::slice::Iter<'a, Warning>;
    type Item = &'a Warning;

    fn into_iter(self) -> Self::IntoIter {
        if let Some(w) = &self.warnings {
            w.iter()
        } else {
            (&[]).iter()
        }
    }
}
