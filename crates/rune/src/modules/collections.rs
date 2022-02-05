//! `std::collections` module.

use crate::runtime::{Iterator, IteratorTrait, Key, Protocol, Ref, Value, VmError, VmErrorKind};
use crate::{Any, ContextError, Module};
use std::fmt;

#[derive(Any, Clone)]
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
    fn extend(&mut self, value: Value) -> Result<(), VmError> {
        use crate::runtime::FromValue;

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
    fn index_get(&self, key: Key) -> Result<Value, VmError> {
        use crate::runtime::TypeOf;

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

    #[inline]
    fn remove(&mut self, key: Key) {
        self.map.remove(&key);
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        use std::fmt::Write;
        write!(s, "{:?}", self.map)
    }
}

#[derive(Any, Clone)]
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
    fn extend(&mut self, value: Value) -> Result<(), VmError> {
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
    fn remove(&mut self, key: Key) {
        self.set.remove(&key);
    }

    #[inline]
    fn difference(&self, other: Ref<HashSet>) -> Iterator {
        Iterator::from(
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
        Iterator::from("std::collections::set::Intersection", intersection)
    }

    #[inline]
    fn union(zelf: Ref<HashSet>, other: Ref<HashSet>) -> Result<Iterator, VmError> {
        // use longest as lead and then append any missing that are in second
        let iter = Union {
            iter: if zelf.len() >= other.len() {
                zelf.iter().chain_raw(other.difference(zelf))?
            } else {
                other.iter().chain_raw(zelf.difference(other))?
            },
        };

        Ok(Iterator::from("std::collections::set::Union", iter))
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        use std::fmt::Write;
        write!(s, "{:?}", self.set)
    }

    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.set == other.set
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

impl IteratorTrait for Union {
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[derive(Any, Clone, Default)]
#[rune(module = "crate")]
struct VecDeque {
    inner: std::collections::VecDeque<Value>,
}

impl VecDeque {
    fn new() -> VecDeque {
        Default::default()
    }

    fn with_capacity(count: usize) -> VecDeque {
        Self {
            inner: std::collections::VecDeque::with_capacity(count),
        }
    }

    /// Extend this VecDeque with something that implements the into_iter
    /// protocol.
    pub fn extend(&mut self, value: Value) -> Result<(), VmError> {
        let mut it = value.into_iter()?;

        while let Some(value) = it.next()? {
            self.push_back(value);
        }

        Ok(())
    }

    fn rotate_left(&mut self, mid: usize) {
        self.inner.rotate_left(mid);
    }

    fn rotate_right(&mut self, mid: usize) {
        self.inner.rotate_left(mid);
    }

    fn push_front(&mut self, v: Value) {
        self.inner.push_front(v);
    }

    fn push_back(&mut self, v: Value) {
        self.inner.push_back(v);
    }

    fn pop_front(&mut self) -> Option<Value> {
        self.inner.pop_front()
    }

    fn pop_back(&mut self) -> Option<Value> {
        self.inner.pop_back()
    }

    fn remove(&mut self, index: usize) {
        self.inner.remove(index);
    }

    fn reserve(&mut self, index: usize) {
        self.inner.reserve(index);
    }

    fn len(&mut self) -> usize {
        self.inner.len()
    }

    fn get(&self, index: usize) -> Result<Value, VmError> {
        if index > self.inner.len() {
            return Err(VmError::from(VmErrorKind::OutOfRange {
                index: index.into(),
                len: self.inner.len().into(),
            }));
        }
        Ok(self.inner[index].clone())
    }

    fn set(&mut self, index: usize, value: Value) -> Result<(), VmError> {
        if index > self.inner.len() {
            return Err(VmError::from(VmErrorKind::OutOfRange {
                index: index.into(),
                len: self.inner.len().into(),
            }));
        }
        self.inner[index] = value;
        Ok(())
    }

    fn insert(&mut self, index: usize, value: Value) {
        self.inner.insert(index, value);
    }

    #[inline]
    fn iter(&self) -> Iterator {
        let iter = self.inner.clone().into_iter();
        Iterator::from("std::collections::VecDeque::Iter", iter)
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        use std::fmt::Write;
        write!(s, "{:?}", self.inner)
    }
}

/// The `std::collections` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["collections"]);
    module.ty::<HashMap>()?;
    module.function(&["HashMap", "new"], HashMap::new)?;
    module.function(&["HashMap", "from"], hashmap_from)?;
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

    module.ty::<HashSet>()?;
    module.function(&["HashSet", "new"], HashSet::new)?;
    module.function(&["HashSet", "from"], hashset_from)?;
    module.inst_fn("clear", HashSet::clear)?;
    module.inst_fn("clone", HashSet::clone)?;
    module.inst_fn("contains", HashSet::contains)?;
    module.inst_fn("difference", HashSet::difference)?;
    module.inst_fn("extend", HashSet::extend)?;
    module.inst_fn("insert", HashSet::insert)?;
    module.inst_fn("intersection", HashSet::intersection)?;
    module.inst_fn("is_empty", HashSet::is_empty)?;
    module.inst_fn("iter", HashSet::iter)?;
    module.inst_fn("len", HashSet::len)?;
    module.inst_fn("remove", HashSet::remove)?;
    module.inst_fn("union", HashSet::union)?;
    module.inst_fn(Protocol::INTO_ITER, HashSet::iter)?;
    module.inst_fn(Protocol::STRING_DEBUG, HashSet::string_debug)?;
    module.inst_fn(Protocol::EQ, HashSet::eq)?;

    module.ty::<VecDeque>()?;
    module.function(&["VecDeque", "new"], VecDeque::new)?;
    module.function(&["VecDeque", "with_capacity"], VecDeque::with_capacity)?;
    module.function(&["VecDeque", "from"], vecdeque_from)?;

    module.inst_fn("extend", VecDeque::extend)?;
    module.inst_fn("insert", VecDeque::insert)?;
    module.inst_fn("iter", VecDeque::iter)?;
    module.inst_fn("len", VecDeque::len)?;
    module.inst_fn("pop_back", VecDeque::pop_back)?;
    module.inst_fn("pop_front", VecDeque::pop_front)?;
    module.inst_fn("push_back", VecDeque::push_back)?;
    module.inst_fn("push_front", VecDeque::push_front)?;
    module.inst_fn("remove", VecDeque::remove)?;
    module.inst_fn("reserve", VecDeque::reserve)?;
    module.inst_fn("rotate_left", VecDeque::rotate_left)?;
    module.inst_fn("rotate_right", VecDeque::rotate_right)?;
    module.inst_fn(Protocol::INDEX_GET, VecDeque::get)?;
    module.inst_fn(Protocol::INDEX_SET, VecDeque::set)?;
    module.inst_fn(Protocol::INTO_ITER, VecDeque::iter)?;
    module.inst_fn(Protocol::STRING_DEBUG, VecDeque::string_debug)?;

    Ok(module)
}

fn hashmap_from(value: Value) -> Result<HashMap, VmError> {
    use crate::runtime::FromValue;

    let mut map = HashMap::new();
    let mut it = value.into_iter()?;

    while let Some(value) = it.next()? {
        let (key, value) = <(Key, Value)>::from_value(value)?;
        map.insert(key, value);
    }

    Ok(map)
}

fn vecdeque_from(value: Value) -> Result<VecDeque, VmError> {
    let mut cont = VecDeque::new();
    let mut it = value.into_iter()?;

    while let Some(value) = it.next()? {
        cont.push_back(value);
    }

    Ok(cont)
}

fn hashset_from(value: Value) -> Result<HashSet, VmError> {
    let mut set = HashSet::new();
    let mut it = value.into_iter()?;

    while let Some(value) = it.next()? {
        set.insert(Key::from_value(&value)?);
    }

    Ok(set)
}
