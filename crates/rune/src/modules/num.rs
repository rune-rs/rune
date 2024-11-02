//! Working with numbers.

use core::num::{ParseFloatError, ParseIntError};

use crate as rune;
use crate::{ContextError, Module};

/// Working with numbers.
///
/// This module provides types generic for working over numbers, such as errors
/// when a number cannot be parsed.
#[rune::module(::std::num)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<ParseFloatError>()?;
    module.ty::<ParseIntError>()?;
    Ok(module)
}
