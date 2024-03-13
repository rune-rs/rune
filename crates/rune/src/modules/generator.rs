//! Generators.

use crate as rune;
use crate::{ContextError, Module};

/// Generators.
///
/// Generator functionality has been moved into [::std::ops].
#[deprecated = "Generators have been moved into std::ops"]
#[rune::module(::std::generator)]
pub fn module() -> Result<Module, ContextError> {
    Module::from_meta(self::module_meta)
}
