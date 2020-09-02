//! Package containing generator functions.

use crate::{ContextError, Generator, Module};

/// Get the module for the array package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "generator"]);
    module.ty(&["Generator"]).build::<Generator>()?;
    module.generator_state(&["GeneratorState"])?;

    module.async_inst_fn("next", Generator::next)?;
    module.async_inst_fn("resume", Generator::resume)?;
    Ok(module)
}
