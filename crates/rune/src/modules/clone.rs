//! The cloning trait for Rune.

use crate as rune;
use crate::runtime::{Value, VmResult};
use crate::{ContextError, Module};

/// The cloning trait for Rune.
///
/// This module defines methods and types used when cloning values.
///
/// By default all values in rune are structurally shared, so in order to get a
/// unique instance of it you must call [`clone`] over it.
#[rune::module(::std::clone)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.function_meta(clone)?;
    Ok(module)
}

/// Clone the specified `value`.
///
/// # Examples
///
/// ```rune
/// let a = 42;
/// let b = a;
/// let c = clone(a);
///
/// a += 1;
/// assert_eq!(a, 43);
/// assert_eq!(b, 43);
/// assert_eq!(c, 42);
/// ```
#[rune::function]
fn clone(value: Value) -> VmResult<Value> {
    value.clone_()
}
