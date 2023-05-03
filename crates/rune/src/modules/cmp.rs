//! The `std::cmp` module.

use core::cmp;

use crate::{ContextError, Module};

/// Construct the `std::cmp` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["cmp"]);
    module.ty::<cmp::Ordering>()?;
    Ok(module)
}
