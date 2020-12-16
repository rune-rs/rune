use runestick::{SourceId, Span};
use std::error;
use std::fmt;
use thiserror::Error;

/// Compilation warning.
#[derive(Debug, Clone, Copy)]
pub struct Warning {
    /// The last warning reported in the chain.
    pub(super) last: Option<usize>,
    /// The id of the source where the warning happened.
    pub(super) source_id: SourceId,
    /// The kind of the warning.
    pub(super) kind: WarningKind,
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
