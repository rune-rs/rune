//! `std::collections` module.

use crate::{Any, ContextError, Iterator, Key, Module, Value};

#[derive(Any)]
#[rune(module = "crate")]
struct HashMap {
    map: crate::collections::HashMap<Key, Value>,
}

impl HashMap {
    fn new() -> Self {
        Self {
            map: crate::collections::HashMap::new(),
        }
    }

    #[inline]
    fn iter(&self) -> Iterator {
        let iter = self.map.clone().into_iter();
        Iterator::from("std::collections::map::Iter", iter)
    }

    #[inline]
    fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    #[inline]
    fn get(&self, key: Key) -> Option<Value> {
        self.map.get(&key).cloned()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    fn clear(&mut self) {
        self.map.clear()
    }
}

#[derive(Any)]
#[rune(module = "crate")]
struct HashSet {
    set: crate::collections::HashSet<Key>,
}

impl HashSet {
    fn new() -> Self {
        Self {
            set: crate::collections::HashSet::new(),
        }
    }

    #[inline]
    fn iter(&self) -> Iterator {
        let iter = self.set.clone().into_iter();
        Iterator::from("std::collections::set::Iter", iter)
    }

    #[inline]
    fn insert(&mut self, key: Key) -> bool {
        self.set.insert(key)
    }

    #[inline]
    fn contains(&self, key: Key) -> bool {
        self.set.contains(&key)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.set.len()
    }

    #[inline]
    fn clear(&mut self) {
        self.set.clear()
    }
}

/// The `std::collections` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["collections"]);
    module.ty::<HashMap>()?;
    module.function(&["HashMap", "new"], HashMap::new)?;
    module.inst_fn("iter", HashMap::iter)?;
    module.inst_fn("insert", HashMap::insert)?;
    module.inst_fn("get", HashMap::get)?;
    module.inst_fn("is_empty", HashMap::is_empty)?;
    module.inst_fn("len", HashMap::len)?;
    module.inst_fn("clear", HashMap::clear)?;
    module.inst_fn(crate::Protocol::INTO_ITER, HashMap::iter)?;
    module.inst_fn(crate::Protocol::INDEX_SET, HashMap::insert)?;
    module.inst_fn(crate::Protocol::INDEX_GET, HashMap::get)?;

    module.ty::<HashSet>()?;
    module.function(&["HashSet", "new"], HashSet::new)?;
    module.inst_fn("iter", HashSet::iter)?;
    module.inst_fn("insert", HashSet::insert)?;
    module.inst_fn("contains", HashSet::contains)?;
    module.inst_fn("is_empty", HashSet::is_empty)?;
    module.inst_fn("len", HashSet::len)?;
    module.inst_fn("clear", HashSet::clear)?;
    module.inst_fn(crate::Protocol::INTO_ITER, HashSet::iter)?;
    Ok(module)
}
