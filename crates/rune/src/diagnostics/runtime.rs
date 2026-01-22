use core::fmt;

use crate::Hash;

/// Runtime Warning diagnostic emitted during the execution of the VM. Warning diagnostics indicates
/// an recoverable issues.
#[derive(Debug)]
pub struct RuntimeDiagnostic {
    /// The instruction pointer of the vm where the warning happened.
    pub(crate) ip: usize,
    /// The kind of the warning.
    pub(crate) kind: RuntimeDiagnosticKind,
}

impl RuntimeDiagnostic {
    /// The instruction pointer of the vm where the warning happened.
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// The kind of the warning.
    #[cfg(feature = "emit")]
    #[allow(unused)]
    pub(crate) fn kind(&self) -> &RuntimeDiagnosticKind {
        &self.kind
    }

    #[cfg(test)]
    #[allow(unused)]
    pub(crate) fn into_kind(self) -> RuntimeDiagnosticKind {
        self.kind
    }
}

impl fmt::Display for RuntimeDiagnostic {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl core::error::Error for RuntimeDiagnostic {}

/// The kind of a [RuntimeWarningDiagnostic].
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum RuntimeDiagnosticKind {
    UsedDeprecated {
        /// The hash which produced the deprecation
        #[cfg_attr(not(feature = "emit"), allow(dead_code))]
        hash: Hash,
    },
}

impl fmt::Display for RuntimeDiagnosticKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RuntimeDiagnosticKind::UsedDeprecated { .. } => {
                write!(f, "Used deprecated function")
            }
        }
    }
}
