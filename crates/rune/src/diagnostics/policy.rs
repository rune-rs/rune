use core::fmt;

use crate::ast::{Span, Spanned};

#[derive(Debug, Spanned)]
#[rune(crate)]
pub(crate) struct PolicyDiagnostic {
    /// The span of the pattern.
    #[rune(span)]
    pub(crate) span: Span,
    /// The context in which it is used.
    #[cfg_attr(not(feature = "emit"), allow(dead_code))]
    pub(crate) context: Option<Span>,
    /// The kind of the policy diagnostic.
    pub(crate) kind: PolicyDiagnosticKind,
}

/// A policy diagnostic.
#[derive(Debug)]
pub(crate) enum PolicyDiagnosticKind {
    /// Warning that an unconditional let pattern will panic if it doesn't
    /// match.
    PatternMightPanic,
}

impl fmt::Display for PolicyDiagnostic {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            PolicyDiagnosticKind::PatternMightPanic { .. } => {
                write!(f, "Pattern might panic if it doesn't match")
            }
        }
    }
}
