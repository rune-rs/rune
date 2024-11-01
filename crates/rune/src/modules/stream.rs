//! Asynchronous streams.

use crate as rune;
use crate::runtime::Stream;
use crate::{ContextError, Module};

/// Asynchronous streams.
#[rune::module(::std::stream)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;
    m.ty::<Stream>()?;
    m.function_meta(Stream::next_shared__meta)?;
    m.function_meta(Stream::resume_shared__meta)?;
    m.function_meta(Stream::debug__meta)?;
    m.function_meta(Stream::clone__meta)?;
    m.implement_trait::<Stream>(rune::item!(::std::clone::Clone))?;
    Ok(m)
}
