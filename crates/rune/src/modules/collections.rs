//! Dynamic collections.

#[cfg(feature = "alloc")]
mod hash_map;
#[cfg(feature = "alloc")]
mod hash_set;
mod vec_deque;

use crate::{ContextError, Module};

#[cfg(feature = "alloc")]
pub(crate) use self::hash_map::HashMap;
#[cfg(feature = "alloc")]
pub(crate) use self::hash_set::HashSet;
pub(crate) use self::vec_deque::VecDeque;
use crate as rune;

/// Dynamic collections.
#[rune::module(::std::collections)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    #[cfg(feature = "alloc")]
    hash_map::setup(&mut module)?;
    #[cfg(feature = "alloc")]
    hash_set::setup(&mut module)?;
    vec_deque::setup(&mut module)?;
    Ok(module)
}
