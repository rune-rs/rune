use core::fmt;

use crate::Hash;

/// Runtime Warning diagnostic emitted during the execution of the VM. Warning diagnostics indicates
/// an recoverable issues.
#[derive(Debug)]
pub struct RuntimeWarningDiagnostic {
    /// The instruction pointer of the vm where the warning happened.
    pub(crate) ip: usize,
    /// The kind of the warning.
    pub(crate) kind: RuntimeWarningDiagnosticKind,
}

impl RuntimeWarningDiagnostic {
    /// The instruction pointer of the vm where the warning happened.
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// The kind of the warning.
    #[cfg(feature = "emit")]
    #[allow(unused)]
    pub(crate) fn kind(&self) -> &RuntimeWarningDiagnosticKind {
        &self.kind
    }

    #[cfg(test)]
    #[allow(unused)]
    pub(crate) fn into_kind(self) -> RuntimeWarningDiagnosticKind {
        self.kind
    }
}

impl fmt::Display for RuntimeWarningDiagnostic {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

cfg_std! {
    impl std::error::Error for RuntimeWarningDiagnostic {
        #[inline]
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            None
        }
    }
}

/// The kind of a [RuntimeWarningDiagnostic].
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum RuntimeWarningDiagnosticKind {
    UsedDeprecated {
        /// The hash which produced the deprecation
        hash: Hash,
    },
}

impl fmt::Display for RuntimeWarningDiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RuntimeWarningDiagnosticKind::UsedDeprecated { .. } => {
                write!(f, "Used deprecated function")
            }
        }
    }
}
