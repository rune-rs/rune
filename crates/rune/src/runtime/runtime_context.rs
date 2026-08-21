use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::String;
use crate::hash;
use crate::runtime::{ConstConstructImpl, ConstContext, ConstValue, ConstValueBuf};
use crate::Hash;

use super::FunctionHandler;

/// Static run context visible to the virtual machine.
///
/// This contains:
/// * Declared functions.
/// * Declared instance functions.
/// * Built-in type checks.
#[derive(Default, TryClone)]
pub struct RuntimeContext {
    /// Registered native function handlers.
    functions: hash::Map<FunctionHandler>,
    /// Named constant values
    constants: hash::Map<ConstValueBuf>,
    /// Constant constructors.
    construct: hash::Map<ConstConstructImpl>,
    /// Registered deprecation messages for native functions.
    deprecations: hash::Map<String>,
}

assert_impl!(RuntimeContext: Send + Sync);

impl RuntimeContext {
    pub(crate) fn new(
        functions: hash::Map<FunctionHandler>,
        constants: hash::Map<ConstValueBuf>,
        construct: hash::Map<ConstConstructImpl>,
        deprecations: hash::Map<String>,
    ) -> Self {
        Self {
            functions,
            constants,
            construct,
            deprecations,
        }
    }

    /// Lookup the given native function handler in the context.
    #[inline]
    pub fn function(&self, hash: &Hash) -> Option<&FunctionHandler> {
        self.functions.get(hash)
    }

    /// Read a constant value.
    #[inline]
    pub fn constant(&self, hash: &Hash) -> Option<&ConstValue> {
        Some(self.constants.get(hash)?)
    }

    /// Read a constant constructor.
    #[inline]
    pub(crate) fn construct(&self, hash: &Hash) -> Option<&ConstConstructImpl> {
        self.construct.get(hash)
    }

    /// Look up the deprecation message associated with a native function, if
    /// the function has been marked as deprecated.
    #[inline]
    pub fn deprecation(&self, hash: &Hash) -> Option<&str> {
        self.deprecations.get(hash).map(String::as_str)
    }
}

impl ConstContext for RuntimeContext {
    #[inline]
    fn get(&self, hash: Hash) -> Option<&ConstConstructImpl> {
        self.construct(&hash)
    }
}

impl fmt::Debug for RuntimeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RuntimeContext")
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(RuntimeContext: Send, Sync);
