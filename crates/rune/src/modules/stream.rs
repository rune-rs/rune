//! The `std::stream` module.

use crate::runtime::Stream;
use crate::{ContextError, Module};

/// Construct the `std::stream` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["stream"]);
    module.ty::<Stream>()?;
    module.async_inst_fn("next", Stream::next)?;
    module.async_inst_fn("resume", Stream::resume)?;
    Ok(module)
}
