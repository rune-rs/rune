//! The `std::char` module.

use crate::{ContextError, Module, Value, VmError, VmErrorKind};
use std::char::ParseCharError;

/// Construct the `std::char` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["char"]);
    module.ty::<ParseCharError>()?;

    module.function(&["from_int"], char_from_int_impl)?;
    module.function(&["is_alphabetic"], char::is_alphabetic)?;
    module.function(&["is_alphanumeric"], char::is_alphanumeric)?;
    module.function(&["is_control"], char::is_control)?;
    module.function(&["is_lowercase"], char::is_lowercase)?;
    module.function(&["is_numeric"], char::is_numeric)?;
    module.function(&["is_uppercase"], char::is_uppercase)?;
    module.function(&["is_whitespace"], char::is_whitespace)?;

    module.function(&["to_digit"], char::to_digit)?;

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
