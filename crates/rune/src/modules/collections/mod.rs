//! Dynamic collections.

#[cfg(feature = "alloc")]
pub(crate) mod hash_map;
#[cfg(feature = "alloc")]
pub(crate) use hash_map::HashMap;

#[cfg(feature = "alloc")]
pub(crate) mod hash_set;
#[cfg(feature = "alloc")]
pub(crate) use hash_set::HashSet;

#[cfg(feature = "alloc")]
pub(crate) mod vec_deque;
#[cfg(feature = "alloc")]
pub(crate) use vec_deque::VecDeque;

use crate as rune;
use crate::{ContextError, Module};

/// Module defining collections.
#[rune::module(::std::collections)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    m.reexport(
        ["HashMap"],
        rune::item!(::std::collections::hash_map::HashMap),
    )?;

    m.reexport(
        ["HashSet"],
        rune::item!(::std::collections::hash_set::HashSet),
    )?;

    m.reexport(
        ["VecDeque"],
        rune::item!(::std::collections::vec_deque::VecDeque),
    )?;

    Ok(m)
}
