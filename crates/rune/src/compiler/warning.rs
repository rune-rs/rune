use st::unit::Span;

/// Compilation warning.
#[derive(Debug, Clone, Copy)]
pub enum Warning {
    /// Item identified by the span is not used.
    NotUsed {
        /// The span that is not used.
        span: Span,
        /// The context in which the value was not used.
        context: Option<Span>,
    },
    /// Warning that an unconditional let pattern will panic if it doesn't
    /// match.
    LetPatternMightPanic {
        /// The span of the pattern.
        span: Span,
        /// The context in which it is used.
        context: Option<Span>,
    },
}
/// Compilation warnings.
#[derive(Debug, Clone, Default)]
pub struct Warnings {
    warnings: Vec<Warning>,
}

impl Warnings {
    /// Construct a new collection of compilation warnings.
    pub(super) fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Indicate if there are warnings or not.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Construct a warning indicating that the item identified by the span is
    /// not used.
    pub(super) fn not_used(&mut self, span: Span, context: Option<Span>) {
        self.warnings.push(Warning::NotUsed { span, context });
    }

    /// Indicate that a pattern might panic.
    pub(super) fn let_pattern_might_panic(&mut self, span: Span, context: Option<Span>) {
        self.warnings
            .push(Warning::LetPatternMightPanic { span, context });
    }
}

impl IntoIterator for Warnings {
    type IntoIter = std::vec::IntoIter<Warning>;
    type Item = Warning;

    fn into_iter(self) -> Self::IntoIter {
        self.warnings.into_iter()
    }
}
