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
