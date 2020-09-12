//! The `std::generator` module.

use crate::{ContextError, Generator, Module};

/// Construct the `std::generator` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "generator"]);
    module.ty::<Generator>()?;
    module.generator_state(&["GeneratorState"])?;

    module.inst_fn("next", Generator::next)?;
    module.inst_fn("resume", Generator::resume)?;
    Ok(module)
}
