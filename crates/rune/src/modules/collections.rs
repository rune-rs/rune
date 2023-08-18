//! `std::collections` module.

mod hash_map;
mod hash_set;
mod vec_deque;

pub(crate) use self::vec_deque::VecDeque;
use crate::{ContextError, Module};

/// The `std::collections` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["collections"]);
    hash_map::setup(&mut module)?;
    hash_set::setup(&mut module)?;
    vec_deque::setup(&mut module)?;
    Ok(module)
}
