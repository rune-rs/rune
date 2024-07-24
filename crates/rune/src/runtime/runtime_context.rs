use core::fmt;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::hash;
use crate::runtime::{ConstValue, InstAddress, Output, Stack, VmResult};
use crate::Hash;

/// A type-reduced function handler.
pub(crate) type FunctionHandler =
    dyn Fn(&mut Stack, InstAddress, usize, Output) -> VmResult<()> + Send + Sync;

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
}

impl RuntimeContext {
    pub(crate) fn new(
        functions: hash::Map<Arc<FunctionHandler>>,
        constants: hash::Map<ConstValue>,
    ) -> Self {
        Self {
            functions,
            constants,
        }
    }

    /// Lookup the given native function handler in the context.
    pub fn function(&self, hash: Hash) -> Option<&Arc<FunctionHandler>> {
        self.functions.get(&hash)
    }

    /// Read a constant value from the unit.
    pub fn constant(&self, hash: Hash) -> Option<&ConstValue> {
        self.constants.get(&hash)
    }
}

impl fmt::Debug for RuntimeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RuntimeContext")
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(RuntimeContext: Send, Sync);
