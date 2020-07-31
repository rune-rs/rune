//! The `int` package.
//!
//! Contains functions such as:
//! * `parse` to parse a string into a number.

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

/// Convert a float to a whole number.
fn to_integer(value: f64) -> i64 {
    value as i64
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["int"]);
    module.fallible_free_fn("parse", parse)?;
    module.inst_fn("to_float", to_float)?;
    module.inst_fn("to_integer", to_integer)?;
    Ok(module)
}
