//! The `std::char` module.

use crate::{ContextError, Module};
use std::char::ParseCharError;

/// Construct the `std::char` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["char"]);
    module.ty::<ParseCharError>()?;
    Ok(module)
}

crate::__internal_impl_any!(ParseCharError);
