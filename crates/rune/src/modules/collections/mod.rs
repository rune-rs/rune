//! Dynamic collections.

pub(crate) mod hash_map;
pub(crate) use hash_map::HashMap;

pub(crate) mod hash_set;
pub(crate) use hash_set::HashSet;

pub(crate) mod vec_deque;
pub(crate) use vec_deque::VecDeque;

use crate as rune;
use crate::{ContextError, Module};

/// Module defining collections.
#[rune::module(::std::collections)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;

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
