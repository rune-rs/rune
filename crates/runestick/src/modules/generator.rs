//! The `std::generator` module.

use crate::{ContextError, Generator, Module};

/// Construct the `std::generator` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "generator"]);
    module.ty(&["Generator"]).build::<Generator>()?;
    module.generator_state(&["GeneratorState"])?;

    module.async_inst_fn("next", Generator::next)?;
    module.async_inst_fn("resume", Generator::resume)?;
    Ok(module)
}
