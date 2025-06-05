use core::fmt;

use rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::hash;
use crate::runtime::{ConstConstruct, ConstValue, InstAddress, Memory, Output, VmError};
use crate::Hash;

/// A type-reduced function handler.
pub(crate) type FunctionHandler =
    dyn Fn(&mut dyn Memory, InstAddress, usize, Output) -> Result<(), VmError> + Send + Sync;

/// Static run context visible to the virtual machine.
///
/// This contains:
/// * Declared functions.
/// * Declared instance functions.
/// * Built-in type checks.
#[derive(Default, TryClone)]
pub struct RuntimeContext {
    /// Registered native function handlers.
    functions: hash::Map<Arc<FunctionHandler>>,
    /// Named constant values
    constants: hash::Map<ConstValue>,
    /// Constant constructors.
    construct: hash::Map<Arc<dyn ConstConstruct>>,
}

assert_impl!(RuntimeContext: Send + Sync);

impl RuntimeContext {
    pub(crate) fn new(
        functions: hash::Map<Arc<FunctionHandler>>,
        constants: hash::Map<ConstValue>,
        construct: hash::Map<Arc<dyn ConstConstruct>>,
    ) -> Self {
        Self {
            functions,
            constants,
            construct,
        }
    }

    /// Lookup the given native function handler in the context.
    #[inline]
    pub fn function(&self, hash: &Hash) -> Option<&Arc<FunctionHandler>> {
        self.functions.get(hash)
    }

    /// Read a constant value.
    #[inline]
    pub fn constant(&self, hash: &Hash) -> Option<&ConstValue> {
        self.constants.get(hash)
    }

    /// Read a constant constructor.
    #[inline]
    pub(crate) fn construct(&self, hash: &Hash) -> Option<&dyn ConstConstruct> {
        Some(&**self.construct.get(hash)?)
    }
}

impl fmt::Debug for RuntimeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RuntimeContext")
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(RuntimeContext: Send, Sync);
