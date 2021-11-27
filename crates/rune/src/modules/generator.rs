//! The `std::generator` module.

use crate::runtime::{Generator, Protocol};
use crate::{ContextError, Module};

/// Construct the `std::generator` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["generator"]);
    module.ty::<Generator>()?;
    module.generator_state(&["GeneratorState"])?;

    module.inst_fn("next", Generator::next)?;
    module.inst_fn("resume", Generator::resume)?;
    module.inst_fn("iter", Generator::into_iterator)?;
    module.inst_fn(Protocol::INTO_ITER, Generator::into_iterator)?;

    Ok(module)
}
