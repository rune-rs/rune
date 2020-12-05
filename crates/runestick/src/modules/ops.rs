//! The `std::ops` module.

use crate::{ContextError, Module, Range};

/// Construct the `std::ops` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["ops"]);
    module.ty::<Range>()?;
    module.inst_fn("contains_int", Range::contains_int)?;
    Ok(module)
}
