//! The `test` package.
//!
//! Contains functions such as:
//! * `assert` assert that a value is true.

use crate::context::{ContextError, Module};
use crate::vm::VmError;

/// Assert that a value is true.
fn assert(value: bool, message: &str) -> Result<(), VmError> {
    if !value {
        return Err(VmError::custom_panic(format!(
            "assertion failed: {}",
            message
        )));
    }

    Ok(())
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "test"]);
    module.function(&["assert"], assert)?;
    Ok(module)
}
