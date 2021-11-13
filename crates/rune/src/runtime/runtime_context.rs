use crate::collections::HashMap;
use crate::macros::{MacroContext, TokenStream};
use crate::runtime::{ConstValue, Stack, TypeCheck, VmError};
use crate::{Hash, Item};
use std::fmt;
use std::sync::Arc;

/// A type-reduced function handler.
pub(crate) type FunctionHandler = dyn Fn(&mut Stack, usize) -> Result<(), VmError> + Send + Sync;

/// A (type erased) macro handler.
pub(crate) type MacroHandler =
    dyn Fn(&mut MacroContext, &TokenStream) -> crate::Result<TokenStream> + Send + Sync;

/// Static run context visible to the virtual machine.
///
/// This contains:
/// * Declared functions.
/// * Declared instance functions.
/// * Built-in type checks.
#[derive(Default)]
pub struct RuntimeContext {
    /// Registered native function handlers.
    pub(crate) functions: HashMap<Hash, Arc<FunctionHandler>>,

    /// Registered types.
    pub(crate) types: HashMap<Hash, TypeCheck>,

    /// Named constant values
    pub(crate) constants: HashMap<Hash, ConstValue>,
}

impl RuntimeContext {
    /// Construct a new empty collection of functions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Use the specified type check.
    pub fn type_check_for(&self, item: &Item) -> Option<TypeCheck> {
        Some(*self.types.get(&Hash::type_hash(item))?)
    }

    /// Lookup the given native function handler in the context.
    pub fn lookup(&self, hash: Hash) -> Option<&Arc<FunctionHandler>> {
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
