//! The `std::hash` module.

use crate as rune;
#[cfg(feature = "std")]
use crate::runtime::Hasher;
use crate::{ContextError, Module};

#[rune::module(::std::hash)]
/// Types for dealing with hashing in Rune.
pub fn module() -> Result<Module, ContextError> {
    #[allow(unused_mut)]
    let mut module = Module::from_meta(self::module_meta);
    #[cfg(feature = "std")]
    module.ty::<Hasher>()?;
    Ok(module)
}
