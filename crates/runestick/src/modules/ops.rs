//! The `std::ops` module.

use crate::{ContextError, Module};

/// Construct the `std::ops` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["ops"]);
    module.ty::<crate::Range>()?;
    Ok(module)
}
