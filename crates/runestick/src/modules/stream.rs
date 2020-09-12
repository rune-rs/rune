//! The `std::stream` module.

use crate::{ContextError, Module, Stream};

/// Construct the `std::stream` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "stream"]);
    module.ty::<Stream>()?;

    module.async_inst_fn("next", Stream::next)?;
    module.async_inst_fn("resume", Stream::resume)?;
    Ok(module)
}
