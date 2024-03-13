//! Working with memory.

use crate as rune;
use crate::runtime::{Value, VmResult};
use crate::{ContextError, Module};

/// Working with memory.
#[rune::module(::std::mem)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
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
    vm_try!(value.drop());
    VmResult::Ok(())
}
