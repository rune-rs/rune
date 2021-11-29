//! The `std::stream` module.

use crate::runtime::{Stream, Vm};
use crate::{ContextError, Module};

/// Construct the `std::stream` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["stream"]);
    module.ty::<Stream<Vm>>()?;
    module.async_inst_fn("next", Stream::<Vm>::next)?;
    module.async_inst_fn("resume", Stream::<Vm>::resume)?;
    Ok(module)
}
