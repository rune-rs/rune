//! The `std::mem` module.

use crate as rune;
use crate::runtime::{Value, VmError};
use crate::{ContextError, Module};

/// Construct the `std` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["mem"]);
    module.function_meta(drop)?;
    Ok(module)
}

#[rune::function]
/// Explicitly drop the given value, freeing up any memory associated with it.
///
/// Normally values are dropped as they go out of scope, but with this method it
/// can be explicitly controlled instead.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let v = [1, 2, 3];
/// drop(v);
/// ```
fn drop(value: Value) -> Result<(), VmError> {
    value.take()?;
    Ok(())
}
