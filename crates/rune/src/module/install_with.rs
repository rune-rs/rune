use core::cmp::Ordering;

use crate::{ContextError, Hash};

use super::Module;

/// Trait to handle the installation of auxilliary functions for a type
/// installed into a module.
pub trait InstallWith {
    /// Hook to install more things into the module.
    fn install_with(_: &mut Module) -> Result<(), ContextError> {
        Ok(())
    }
}

impl InstallWith for i64 {}
impl InstallWith for u64 {}
impl InstallWith for f64 {}
impl InstallWith for char {}
impl InstallWith for bool {}
impl InstallWith for Ordering {}
impl InstallWith for Hash {}
