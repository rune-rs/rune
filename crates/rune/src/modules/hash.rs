//! Hashing types.

use crate as rune;
use crate::runtime::Hasher;
use crate::{ContextError, Module};

/// Hashing types.
#[rune::module(::std::hash)]
pub fn module() -> Result<Module, ContextError> {
    #[allow(unused_mut)]
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Hasher>()?;
    Ok(module)
}
