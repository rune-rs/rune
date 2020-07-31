//! The `int` package.
//!
//! Contains functions such as:
//! * `parse` to parse a string into a number.

use crate::error::Result;
use crate::functions::{Module, RegisterError};

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
pub fn module() -> Result<Module, RegisterError> {
    let mut module = Module::new(&["int"]);
    module.global_fallible_fn("parse", parse)?;
    module.instance_fn("to_float", to_float)?;
    module.instance_fn("to_integer", to_integer)?;
    Ok(module)
}
