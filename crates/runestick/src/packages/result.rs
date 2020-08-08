//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::ValuePtr;

/// Construct an error result.
fn error(value: ValuePtr) -> Result<ValuePtr, ValuePtr> {
    Err(value)
}

/// Construct an ok result.
fn ok(value: ValuePtr) -> Result<ValuePtr, ValuePtr> {
    Ok(value)
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "result"]);
    module.function(&["error"], error)?;
    module.function(&["ok"], ok)?;
    Ok(module)
}
