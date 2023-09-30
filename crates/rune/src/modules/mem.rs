//! The `std::mem` module.

use crate as rune;
use crate::runtime::{Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["mem"])?;
    module.function_meta(drop)?;
    Ok(module)
}

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
#[rune::function]
fn drop(value: Value) -> VmResult<()> {
    vm_try!(value.take());
    VmResult::Ok(())
}
