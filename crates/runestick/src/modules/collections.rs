//! `std::collections` module.

use crate::{
    Any, ContextError, Interface, Iterator, Key, Module, Ref, Value, VmError, VmErrorKind,
};

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

    /// Extend this hashmap from an iterator.
    #[inline]
    fn extend(&mut self, value: Interface) -> Result<(), VmError> {
        use crate::FromValue as _;

        let mut it = value.into_iter()?;

        while let Some(value) = it.next()? {
            let (key, value) = <(Key, Value)>::from_value(value)?;
            self.map.insert(key, value);
        }

        Ok(())
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
    fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    #[inline]
    fn get(&self, key: Key) -> Option<Value> {
        self.map.get(&key).cloned()
    }

    #[inline]
    fn fallible_get(&self, key: Key) -> Result<Value, VmError> {
        use crate::TypeOf as _;

        let value = self.map.get(&key).ok_or_else(|| {
            VmError::from(VmErrorKind::MissingIndexKey {
                target: Self::type_info(),
                index: key,
            })
        })?;

        Ok(value.clone())
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

    /// Extend this set from an iterator.
    #[inline]
    fn extend(&mut self, value: Interface) -> Result<(), VmError> {
        let mut it = value.into_iter()?;

        while let Some(value) = it.next()? {
            let key = Key::from_value(&value)?;
            self.set.insert(key);
        }

        Ok(())
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

    #[inline]
    fn difference(&self, other: Ref<HashSet>) -> crate::Iterator {
        crate::Iterator::from(
            "std::collections::set::Difference",
            Difference {
                this: self.set.clone().into_iter(),
                other: Some(other),
            },
        )
    }

    #[inline]
    fn intersection(zelf: Ref<HashSet>, other: Ref<HashSet>) -> Iterator {
        // use shortest iterator as driver for intersections
        let intersection = if zelf.len() <= other.len() {
            Intersection {
                this: zelf.set.clone().into_iter(),
                other: Some(other),
            }
        } else {
            Intersection {
                this: other.set.clone().into_iter(),
                other: Some(zelf),
            }
        };
        crate::Iterator::from("std::collections::set::Intersection", intersection)
    }

    #[inline]
    fn union(zelf: Ref<HashSet>, other: Ref<HashSet>) -> Result<crate::Iterator, VmError> {
        // use longest as lead and then append any missing that are in second
        let iter = Union {
            iter: if zelf.len() >= other.len() {
                zelf.iter().chain_raw(other.difference(zelf))?
            } else {
                other.iter().chain_raw(zelf.difference(other))?
            },
        };

        Ok(crate::Iterator::from("std::collections::set::Union", iter))
    }
}

struct Intersection<I>
where
    I: std::iter::Iterator<Item = Key>,
{
    this: I,
    other: Option<Ref<HashSet>>,
}

impl<I> std::iter::Iterator for Intersection<I>
where
    I: std::iter::Iterator<Item = Key>,
{
    type Item = Key;
    fn next(&mut self) -> Option<Self::Item> {
        let other = self.other.take()?;

        loop {
            let item = self.this.next()?;

            if other.set.contains(&item) {
                self.other = Some(other);
                return Some(item);
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

struct Difference<I>
where
    I: std::iter::Iterator<Item = Key>,
{
    this: I,
    other: Option<Ref<HashSet>>,
}

impl<I> std::iter::Iterator for Difference<I>
where
    I: std::iter::Iterator<Item = Key>,
{
    type Item = Key;
    fn next(&mut self) -> Option<Self::Item> {
        let other = self.other.take()?;

        loop {
            let item = self.this.next()?;

            if !other.set.contains(&item) {
                self.other = Some(other);
                return Some(item);
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

struct Union {
    iter: Iterator,
}

impl crate::iterator::IteratorTrait for Union {
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// The `std::collections` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["collections"]);
    module.ty::<HashMap>()?;
    module.function(&["HashMap", "new"], HashMap::new)?;
    module.function(&["HashMap", "from"], hashmap_from)?;
    module.inst_fn("extend", HashMap::extend)?;
    module.inst_fn("iter", HashMap::iter)?;
    module.inst_fn("keys", HashMap::keys)?;
    module.inst_fn("contains_key", HashMap::contains_key)?;
    module.inst_fn("values", HashMap::values)?;
    module.inst_fn("insert", HashMap::insert)?;
    module.inst_fn("get", HashMap::get)?;
    module.inst_fn("is_empty", HashMap::is_empty)?;
    module.inst_fn("len", HashMap::len)?;
    module.inst_fn("clear", HashMap::clear)?;
    module.inst_fn(crate::Protocol::INTO_ITER, HashMap::iter)?;
    module.inst_fn(crate::Protocol::INDEX_SET, HashMap::insert)?;
    module.inst_fn(crate::Protocol::INDEX_GET, HashMap::fallible_get)?;

    module.ty::<HashSet>()?;
    module.function(&["HashSet", "new"], HashSet::new)?;
    module.function(&["HashSet", "from"], hashset_from)?;
    module.inst_fn("extend", HashSet::extend)?;
    module.inst_fn("iter", HashSet::iter)?;
    module.inst_fn("insert", HashSet::insert)?;
    module.inst_fn("contains", HashSet::contains)?;
    module.inst_fn("is_empty", HashSet::is_empty)?;
    module.inst_fn("len", HashSet::len)?;
    module.inst_fn("clear", HashSet::clear)?;
    module.inst_fn("difference", HashSet::difference)?;
    module.inst_fn("intersection", HashSet::intersection)?;
    module.inst_fn("union", HashSet::union)?;
    module.inst_fn(crate::Protocol::INTO_ITER, HashSet::iter)?;
    Ok(module)
}

fn hashmap_from(interface: Interface) -> Result<HashMap, VmError> {
    use crate::FromValue as _;

    let mut map = HashMap::new();
    let mut it = interface.into_iter()?;

    while let Some(value) = it.next()? {
        let (key, value) = <(Key, Value)>::from_value(value)?;
        map.insert(key, value);
    }

    Ok(map)
}

fn hashset_from(interface: Interface) -> Result<HashSet, VmError> {
    let mut set = HashSet::new();
    let mut it = interface.into_iter()?;

    while let Some(value) = it.next()? {
        set.insert(Key::from_value(&value)?);
    }

    Ok(set)
}
