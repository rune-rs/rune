//! `std::collections` module.

mod hash_map;
mod hash_set;
mod vec_deque;

use crate::{ContextError, Module};

pub(crate) use self::hash_map::HashMap;
pub(crate) use self::hash_set::HashSet;
pub(crate) use self::vec_deque::VecDeque;
use crate as rune;

#[rune::module(::std::collections)]
/// The `std::collections` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta);
    #[cfg(feature = "std")]
    hash_map::setup(&mut module)?;
    #[cfg(feature = "std")]
    hash_set::setup(&mut module)?;
    vec_deque::setup(&mut module)?;
    Ok(module)
}
