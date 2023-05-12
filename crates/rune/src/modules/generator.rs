//! The `std::generator` module.

use crate::runtime::{Generator, Protocol, Vm};
use crate::{ContextError, Module};

/// Construct the `std::generator` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["generator"]);
    module.ty::<Generator<Vm>>()?;
    module.associated_function("next", Generator::<Vm>::next)?;
    module.associated_function("resume", Generator::<Vm>::resume)?;
    module.associated_function("iter", Generator::<Vm>::into_iterator)?;
    module.associated_function(Protocol::INTO_ITER, Generator::<Vm>::into_iterator)?;
    module.generator_state(["GeneratorState"])?;
    Ok(module)
}
