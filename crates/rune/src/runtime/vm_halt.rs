use core::fmt;

use crate::runtime::{Awaited, VmCall};

/// The reason why the virtual machine execution stopped.
#[derive(Debug)]
pub(crate) enum VmHalt {
    /// The virtual machine exited by running out of call frames.
    Exited,
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded,
    /// The virtual machine awaited on the given future.
    Awaited(Awaited),
    /// Call into a new virtual machine.
    VmCall(VmCall),
}

impl VmHalt {
    /// Convert into cheap info enum which only described the reason.
    pub(crate) fn into_info(self) -> VmHaltInfo {
        match self {
            Self::Exited => VmHaltInfo::Exited,
            Self::Limited => VmHaltInfo::Limited,
            Self::Yielded => VmHaltInfo::Yielded,
            Self::Awaited(..) => VmHaltInfo::Awaited,
            Self::VmCall(..) => VmHaltInfo::VmCall,
        }
    }
}

/// The reason why the virtual machine execution stopped.
#[derive(Debug, Clone, Copy)]
pub enum VmHaltInfo {
    /// The virtual machine exited by running out of call frames.
    Exited,
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded,
    /// The virtual machine awaited on the given future.
    Awaited,
    /// Received instruction to push the inner virtual machine.
    VmCall,
}

impl fmt::Display for VmHaltInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exited => write!(f, "exited"),
            Self::Limited => write!(f, "limited"),
            Self::Yielded => write!(f, "yielded"),
            Self::Awaited => write!(f, "awaited"),
            Self::VmCall => write!(f, "calling into other vm"),
        }
    }
}
