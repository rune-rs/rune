//! The `std::generator` module.

use crate::{ContextError, Module};

/// Construct the `std::generator` module.
#[deprecated = "Generators have been moved into std::ops"]
pub fn module() -> Result<Module, ContextError> {
    Module::with_crate_item("std", ["generator"])
}
