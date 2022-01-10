//! The `std::generator` module.

use crate::runtime::{Generator, Protocol, Vm};
use crate::{ContextError, Module};

/// Construct the `std::generator` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["generator"]);
    module.ty::<Generator<Vm>>()?;

    module.inst_fn("next", Generator::<Vm>::next)?;
    module.inst_fn("resume", Generator::<Vm>::resume)?;
    module.inst_fn("iter", Generator::<Vm>::into_iterator)?;
    module.inst_fn(Protocol::INTO_ITER, Generator::<Vm>::into_iterator)?;
    module.generator_state(&["GeneratorState"])?;

    Ok(module)
}
