//! `std::collections` module.

use crate::{Any, ContextError, Key, Module, Value};

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

    fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    fn get(&self, key: Key) -> Option<Value> {
        self.map.get(&key).cloned()
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

    fn insert(&mut self, key: Key) -> bool {
        self.set.insert(key)
    }

    fn contains(&self, key: Key) -> bool {
        self.set.contains(&key)
    }
}

/// The `std::collections` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["collections"]);
    module.ty::<HashMap>()?;
    module.function(&["HashMap", "new"], HashMap::new)?;
    module.inst_fn("insert", HashMap::insert)?;
    module.inst_fn(crate::Protocol::INDEX_SET, HashMap::insert)?;
    module.inst_fn("get", HashMap::get)?;
    module.inst_fn(crate::Protocol::INDEX_GET, HashMap::get)?;

    module.ty::<HashSet>()?;
    module.function(&["HashSet", "new"], HashSet::new)?;
    module.inst_fn("insert", HashSet::insert)?;
    module.inst_fn("contains", HashSet::contains)?;
    Ok(module)
}
