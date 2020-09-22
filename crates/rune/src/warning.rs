use crate::Spanned;
use runestick::{SourceId, Span};
use std::error;
use std::fmt;
use thiserror::Error;

/// Compilation warning.
#[derive(Debug, Clone, Copy)]
pub struct Warning {
    /// The id of the source where the warning happened.
    pub source_id: SourceId,
    /// The kind of the warning.
    pub kind: WarningKind,
}

impl Warning {
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
