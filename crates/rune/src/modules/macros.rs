//! Macro support.

pub mod builtin;

use crate as rune;
use crate::{ContextError, Module};

/// Macro support.
#[rune::module(::std::macros)]
pub fn module() -> Result<Module, ContextError> {
    Module::from_meta(self::module__meta)
}
