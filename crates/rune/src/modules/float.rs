//! The `std::float` module.

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

crate::__internal_impl_any!(::std::float, ParseFloatError);

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["float"]);

    module.ty::<ParseFloatError>()?;
    module.function(["parse"], parse)?;
    module.inst_fn("max", f64::max)?;
    module.inst_fn("min", f64::min)?;
    #[cfg(feature = "std")]
    module.inst_fn("abs", f64::abs)?;
    #[cfg(feature = "std")]
    module.inst_fn("powf", f64::powf)?;
    #[cfg(feature = "std")]
    module.inst_fn("powi", f64::powi)?;

    module.inst_fn("to_integer", to_integer)?;

    Ok(module)
}
