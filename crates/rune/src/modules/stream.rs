//! Asynchronous streams.

use crate as rune;
use crate::runtime::{Stream, Vm};
use crate::{ContextError, Module};

/// Asynchronous streams.
#[rune::module(::std::stream)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Stream<Vm>>()?;
    module.associated_function("next", Stream::<Vm>::next_shared)?;
    module.associated_function("resume", Stream::<Vm>::resume_shared)?;
    Ok(module)
}
