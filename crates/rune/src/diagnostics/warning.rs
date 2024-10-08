use core::fmt;

use crate::alloc::String;
use crate::ast::Span;
use crate::ast::Spanned;
use crate::SourceId;

/// Warning diagnostic emitted during compilation. Warning diagnostics indicates
/// an recoverable issues.
#[derive(Debug)]
pub struct WarningDiagnostic {
    /// The id of the source where the warning happened.
    pub(crate) source_id: SourceId,
    /// The kind of the warning.
    pub(crate) kind: WarningDiagnosticKind,
}

impl WarningDiagnostic {
    /// The source id where the warning originates from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// The kind of the warning.
    #[cfg(feature = "emit")]
    pub(crate) fn kind(&self) -> &WarningDiagnosticKind {
        &self.kind
    }

    #[cfg(test)]
    pub(crate) fn into_kind(self) -> WarningDiagnosticKind {
        self.kind
    }

    /// Access context of warning, if any is available.
    #[cfg(feature = "emit")]
    pub(crate) fn context(&self) -> Option<Span> {
        match &self.kind {
            WarningDiagnosticKind::LetPatternMightPanic { context, .. }
            | WarningDiagnosticKind::RemoveTupleCallParams { context, .. }
            | WarningDiagnosticKind::NotUsed { context, .. }
            | WarningDiagnosticKind::UsedDeprecated { context, .. }
            | WarningDiagnosticKind::TemplateWithoutExpansions { context, .. } => *context,
            _ => None,
        }
    }
}

impl Spanned for WarningDiagnostic {
    /// Get the span of the warning.
    fn span(&self) -> Span {
        match &self.kind {
            WarningDiagnosticKind::NotUsed { span, .. } => *span,
            WarningDiagnosticKind::Unreachable { span, .. } => *span,
            WarningDiagnosticKind::LetPatternMightPanic { span, .. } => *span,
            WarningDiagnosticKind::TemplateWithoutExpansions { span, .. } => *span,
            WarningDiagnosticKind::RemoveTupleCallParams { span, .. } => *span,
            WarningDiagnosticKind::UnnecessarySemiColon { span, .. } => *span,
            WarningDiagnosticKind::UsedDeprecated { span, .. } => *span,
        }
    }
}

impl fmt::Display for WarningDiagnostic {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl core::error::Error for WarningDiagnostic {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}

/// The kind of a [WarningDiagnostic].
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum WarningDiagnosticKind {
    /// Item identified by the span is not used.
    NotUsed {
        /// The span that is not used.
        span: Span,
        /// The context in which the value was not used.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        context: Option<Span>,
    },
    /// Unreachable code.
    Unreachable {
        /// The span that is not used.
        span: Span,
        /// The span which caused the code to be unreachable.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        cause: Span,
    },
    /// Warning that an unconditional let pattern will panic if it doesn't
    /// match.
    LetPatternMightPanic {
        /// The span of the pattern.
        span: Span,
        /// The context in which it is used.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        context: Option<Span>,
    },
    /// Encountered a template string without an expansion.
    TemplateWithoutExpansions {
        /// Span that caused the error.
        span: Span,
        /// The context in which it is used.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        context: Option<Span>,
    },
    /// Suggestion that call parameters could be removed.
    RemoveTupleCallParams {
        /// The span of the call.
        span: Span,
        /// The span of the variant being built.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        variant: Span,
        /// The context in which it is used.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        context: Option<Span>,
    },
    /// An unecessary semi-colon is used.
    UnnecessarySemiColon {
        /// Span where the semi-colon is.
        span: Span,
    },
    UsedDeprecated {
        /// The span which is deprecated
        span: Span,
        /// The context in which it is used.
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        context: Option<Span>,
        /// Deprecated message.
        message: String,
    },
}

impl fmt::Display for WarningDiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WarningDiagnosticKind::NotUsed { .. } => write!(f, "Not used"),
            WarningDiagnosticKind::Unreachable { .. } => write!(f, "Unreachable code"),
            WarningDiagnosticKind::LetPatternMightPanic { .. } => {
                write!(f, "Pattern might panic")
            }
            WarningDiagnosticKind::TemplateWithoutExpansions { .. } => write!(
                f,
                "Using a template string without expansions, like `Hello World`"
            ),
            WarningDiagnosticKind::RemoveTupleCallParams { .. } => {
                write!(f, "Call paramters are not needed here")
            }
            WarningDiagnosticKind::UnnecessarySemiColon { .. } => {
                write!(f, "Unnecessary semicolon")
            }
            WarningDiagnosticKind::UsedDeprecated { message, .. } => {
                write!(f, "Used deprecated function: {message}")
            }
        }
    }
}
