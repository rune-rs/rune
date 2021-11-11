//! The `std::int` module.

use crate::{ContextError, Module};
use std::num::ParseIntError;

/// Construct the `std::int` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["int"]);

    module.ty::<ParseIntError>()?;

    module.function(&["parse"], parse)?;
    module.function(&["max"], i64::max)?;
    module.function(&["min"], i64::min)?;
    module.function(&["abs"], i64::abs)?;

    module.inst_fn("to_float", to_float)?;

    module.inst_fn("abs", i64::abs)?;
    module.inst_fn("checked_add", i64::checked_add)?;
    module.inst_fn("checked_sub", i64::checked_sub)?;
    module.inst_fn("checked_div", i64::checked_div)?;
    module.inst_fn("checked_mul", i64::checked_mul)?;
    module.inst_fn("checked_rem", i64::checked_rem)?;

    module.inst_fn("wrapping_add", i64::wrapping_add)?;
    module.inst_fn("wrapping_sub", i64::wrapping_sub)?;
    module.inst_fn("wrapping_div", i64::wrapping_div)?;
    module.inst_fn("wrapping_mul", i64::wrapping_mul)?;
    module.inst_fn("wrapping_rem", i64::wrapping_rem)?;

    module.inst_fn("saturating_add", i64::saturating_add)?;
    module.inst_fn("saturating_sub", i64::saturating_sub)?;
    module.inst_fn("saturating_mul", i64::saturating_mul)?;
    module.inst_fn("saturating_abs", i64::saturating_abs)?;
    module.inst_fn("saturating_pow", i64::saturating_pow)?;

    module.inst_fn("pow", i64::pow)?;
    Ok(module)
}

/// Parse an integer.
fn parse(s: &str) -> Result<i64, ParseIntError> {
    str::parse::<i64>(s)
}

/// Convert a whole number to float.
fn to_float(value: i64) -> f64 {
    value as f64
}

crate::__internal_impl_any!(ParseIntError);
