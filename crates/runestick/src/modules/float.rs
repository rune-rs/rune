//! The `std::float` module.

use crate::{ContextError, Module};
use std::num::ParseFloatError;

/// Parse an integer.
fn parse(s: &str) -> Result<f64, ParseFloatError> {
    Ok(str::parse::<f64>(s)?)
}

/// Convert a float to a whole number.
fn to_integer(value: f64) -> i64 {
    value as i64
}

crate::__internal_impl_any!(ParseFloatError);

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std"]);

    module.ty::<f64>()?;
    module.ty::<ParseFloatError>()?;
    module.function(&["float", "parse"], parse)?;
    module.inst_fn("to_integer", to_integer)?;

    Ok(module)
}
