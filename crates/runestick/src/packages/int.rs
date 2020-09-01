//! The `int` package.
//!
//! Contains functions such as:
//! * `int::parse` to parse a string into a number.

use crate::{ContextError, Module, Result};

/// Parse an integer.
fn parse(s: &str) -> Result<i64> {
    Ok(str::parse::<i64>(s)?)
}

/// Convert a whole number to float.
fn to_float(value: i64) -> f64 {
    value as f64
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.ty(&["int"]).build::<i64>()?;

    module.function(&["int", "parse"], parse)?;
    module.inst_fn("to_float", to_float)?;

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
