//! The `std::num` module.

use core::num::{ParseFloatError, ParseIntError};

use crate::{ContextError, Module};

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["num"])?;
    module.ty::<ParseFloatError>()?;
    module.ty::<ParseIntError>()?;
    Ok(module)
}

crate::__internal_impl_any!(::std::num, ParseFloatError);
crate::__internal_impl_any!(::std::num, ParseIntError);
