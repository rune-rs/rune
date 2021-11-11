//! The `std::mem` module.

use crate::runtime::{Value, VmError};
use crate::{ContextError, Module};

/// Construct the `std` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["mem"]);
    module.function(&["drop"], drop_impl)?;
    Ok(module)
}

fn drop_impl(value: Value) -> Result<(), VmError> {
    value.take()?;
    Ok(())
}
