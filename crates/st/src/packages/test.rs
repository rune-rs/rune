//! The `test` package.
//!
//! Contains functions such as:
//! * `assert` assert that a value is true.

use crate::error::{Error, Result};
use crate::functions::{Module, RegisterError};

/// Assert that a value is true.
fn assert(value: bool, message: &str) -> Result<()> {
    if !value {
        return Err(Error::msg(format!("assertion failed: {}", message)));
    }

    Ok(())
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, RegisterError> {
    let mut module = Module::new(&["test"]);
    module.global_fallible_fn("assert", assert)?;
    Ok(module)
}
