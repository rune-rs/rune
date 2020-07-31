//! The `int` package.
//!
//! Contains functions such as:
//! * `int::parse` to parse a string into a number.

use crate::context::{ContextError, Module};
use crate::error::Result;

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

    module.ty::<i64>("int")?;
    module.fallible_free_fn(&["int", "parse"], parse)?;
    module.inst_fn("to_float", to_float)?;

    Ok(module)
}
