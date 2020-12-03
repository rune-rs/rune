//! The `std::char` module.

use crate::{ContextError, Module, Value, VmError, VmErrorKind};
use std::char::ParseCharError;

/// Construct the `std::char` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["char"]);
    module.ty::<ParseCharError>()?;
    module.function(&["from_int"], char_from_int_impl)?;
    Ok(module)
}

fn char_from_int_impl(value: i64) -> Result<Option<Value>, VmError> {
    if value < 0 {
        Err(VmError::from(VmErrorKind::Underflow))
    } else if value > u32::MAX as i64 {
        Err(VmError::from(VmErrorKind::Overflow))
    } else {
        Ok(std::char::from_u32(value as u32).map(|v| v.into()))
    }
}

crate::__internal_impl_any!(ParseCharError);
