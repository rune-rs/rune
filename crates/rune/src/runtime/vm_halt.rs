use core::fmt;

use crate::runtime::{Address, Awaited, Output, VmCall};

/// The reason why the virtual machine execution stopped.
#[derive(Debug)]
pub(crate) enum VmHalt {
    /// The virtual machine exited by running out of call frames, returning the given value.
    Exited(Option<Address>),
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded(Option<Address>, Output),
    /// The virtual machine awaited on the given future.
    Awaited(Awaited),
    /// Call into a new virtual machine.
    VmCall(VmCall),
}

/// The reason why the virtual machine execution stopped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VmHaltInfo {
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded,
    /// The virtual machine awaited on the given future.
    Awaited,
}

impl fmt::Display for VmHaltInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limited => write!(f, "limited"),
            Self::Yielded => write!(f, "yielded"),
            Self::Awaited => write!(f, "awaited"),
        }
    }
}
