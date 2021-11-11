//! The `std::cmp` module.

use crate::{ContextError, Module};

/// Construct the `std::cmp` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["cmp"]);

    module.ty::<std::cmp::Ordering>()?;

    Ok(module)
}
