//! The `std::stream` module.

use crate::runtime::{Stream, Vm};
use crate::{ContextError, Module};

/// Construct the `std::stream` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["stream"])?;
    module.ty::<Stream<Vm>>()?;
    module.associated_function("next", Stream::<Vm>::next_shared)?;
    module.associated_function("resume", Stream::<Vm>::resume_shared)?;
    Ok(module)
}
