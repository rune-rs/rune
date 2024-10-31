//! Asynchronous streams.

use crate as rune;
use crate::runtime::Stream;
use crate::{ContextError, Module};

/// Asynchronous streams.
#[rune::module(::std::stream)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Stream>()?;
    module.associated_function("next", Stream::next_shared)?;
    module.associated_function("resume", Stream::resume_shared)?;
    Ok(module)
}
