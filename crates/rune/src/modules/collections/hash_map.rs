use core::fmt::{self, Write};

use crate::no_std::collections;
use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{Iterator, Key, Protocol, Value, VmErrorKind, VmResult};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<HashMap>()?;
    module.function_meta(HashMap::__new__meta)?;
    module.function_meta(hashmap_from)?;
    module.inst_fn("clear", HashMap::clear)?;
    module.inst_fn("clone", HashMap::clone)?;
    module.inst_fn("contains_key", HashMap::contains_key)?;
    module.inst_fn("extend", HashMap::extend)?;
    module.inst_fn("get", HashMap::get)?;
    module.inst_fn("insert", HashMap::insert)?;
    module.inst_fn("is_empty", HashMap::is_empty)?;
    module.inst_fn("iter", HashMap::iter)?;
    module.inst_fn("keys", HashMap::keys)?;
    module.inst_fn("len", HashMap::len)?;
    module.inst_fn("remove", HashMap::remove)?;
    module.inst_fn("values", HashMap::values)?;
    module.inst_fn(Protocol::INTO_ITER, HashMap::iter)?;
    module.inst_fn(Protocol::INDEX_SET, HashMap::index_set)?;
    module.inst_fn(Protocol::INDEX_GET, HashMap::index_get)?;
    module.inst_fn(Protocol::STRING_DEBUG, HashMap::string_debug)?;
    Ok(())
}

#[derive(Any, Clone)]
#[rune(module = crate, item = ::std::collections)]
struct HashMap {
    map: collections::HashMap<Key, Value>,
}

impl HashMap {
    /// Construct a new map.
    #[rune::function(keep, path = Self::new)]
    fn new() -> Self {
        Self {
            map: collections::HashMap::new(),
        }
    }

    /// Extend this hashmap from an iterator.
    #[inline]
    fn extend(&mut self, value: Value) -> VmResult<()> {
        use crate::runtime::FromValue;

        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            let (key, value) = vm_try!(<(Key, Value)>::from_value(value));
            self.map.insert(key, value);
        }

        VmResult::Ok(())
    }

    #[inline]
    fn iter(&self) -> Iterator {
        let iter = self.map.clone().into_iter();
        Iterator::from("std::collections::map::Iter", iter)
    }

    #[inline]
    fn keys(&self) -> Iterator {
        let iter = self.map.keys().cloned().collect::<Vec<_>>().into_iter();
        Iterator::from("std::collections::map::Keys", iter)
    }

    #[inline]
    fn values(&self) -> Iterator {
        let iter = self.map.values().cloned().collect::<Vec<_>>().into_iter();
        Iterator::from("std::collections::map::Values", iter)
    }

    #[inline]
    fn contains_key(&self, key: Key) -> bool {
        self.map.contains_key(&key)
    }

    #[inline]
    fn index_set(&mut self, key: Key, value: Value) {
        let _ = self.map.insert(key, value);
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
    fn index_get(&self, key: Key) -> VmResult<Value> {
        use crate::runtime::TypeOf;

        let value = vm_try!(self.map.get(&key).ok_or_else(|| {
            VmErrorKind::MissingIndexKey {
                target: Self::type_info(),
                index: key,
            }
        }));

        VmResult::Ok(value.clone())
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

    #[inline]
    fn remove(&mut self, key: Key) {
        self.map.remove(&key);
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        write!(s, "{:?}", self.map)
    }
}

/// Convert a hashmap from a `value`.
///
/// The hashmap can be converted from anything that implements the `into_iter`
/// protocol, and each item produces should be a tuple pair.
#[rune::function(path = HashMap::from)]
fn hashmap_from(value: Value) -> VmResult<HashMap> {
    use crate::runtime::FromValue;

    let mut map = HashMap::new();
    let mut it = vm_try!(value.into_iter());

    while let Some(value) = vm_try!(it.next()) {
        let (key, value) = vm_try!(<(Key, Value)>::from_value(value));
        map.insert(key, value);
    }

    VmResult::Ok(map)
}
