//! The `std::f64` module.

use core::num::ParseFloatError;

use crate::{ContextError, Module};

/// Parse an integer.
fn parse(s: &str) -> Result<f64, ParseFloatError> {
    str::parse::<f64>(s)
}

/// Convert a float to a whole number.
fn to_integer(value: f64) -> i64 {
    value as i64
}

crate::__internal_impl_any!(::std::f64, ParseFloatError);

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["f64"]);

    module.ty::<ParseFloatError>()?;
    module.function(["parse"], parse)?;
    module.associated_function("max", f64::max)?;
    module.associated_function("min", f64::min)?;
    #[cfg(feature = "std")]
    module.associated_function("abs", f64::abs)?;
    #[cfg(feature = "std")]
    module.associated_function("powf", f64::powf)?;
    #[cfg(feature = "std")]
    module.associated_function("powi", f64::powi)?;
    module.associated_function("to_integer", to_integer)?;
    Ok(module)
}
