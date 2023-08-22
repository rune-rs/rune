use core::fmt::{self, Write};

use crate::no_std::collections;
use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{
    EnvProtocolCaller, FromValue, Iterator, Key, ProtocolCaller, Value, VmErrorKind, VmResult,
};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<HashMap>()?;
    module.function_meta(HashMap::new)?;
    module.function_meta(HashMap::with_capacity)?;
    module.function_meta(HashMap::len)?;
    module.function_meta(HashMap::insert)?;
    module.function_meta(HashMap::get)?;
    module.function_meta(HashMap::contains_key)?;
    module.function_meta(HashMap::remove)?;
    module.function_meta(HashMap::clear)?;
    module.function_meta(HashMap::is_empty)?;
    module.function_meta(HashMap::iter)?;
    module.function_meta(HashMap::keys)?;
    module.function_meta(HashMap::values)?;
    module.function_meta(HashMap::extend)?;
    module.function_meta(from)?;
    module.function_meta(clone)?;
    module.function_meta(HashMap::into_iter)?;
    module.function_meta(HashMap::index_set)?;
    module.function_meta(HashMap::index_get)?;
    module.function_meta(HashMap::string_debug)?;
    module.function_meta(HashMap::partial_eq)?;
    module.function_meta(HashMap::eq)?;
    Ok(())
}

#[derive(Any, Clone)]
#[rune(module = crate, item = ::std::collections)]
pub(crate) struct HashMap {
    map: collections::HashMap<Key, Value>,
}

impl HashMap {
    /// Creates an empty `HashMap`.
    ///
    /// The hash map is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    /// let map = HashMap::new();
    /// ```
    #[rune::function(path = Self::new)]
    fn new() -> Self {
        Self {
            map: collections::HashMap::new(),
        }
    }

    /// Creates an empty `HashMap` with at least the specified capacity.
    ///
    /// The hash map will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the hash map will not allocate.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    /// let map = HashMap::with_capacity(10);
    /// ```
    #[rune::function(path = Self::with_capacity)]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            map: collections::HashMap::with_capacity(capacity),
        }
    }

    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let a = HashMap::new();
    /// assert_eq!(a.len(), 0);
    /// a.insert(1, "a");
    /// assert_eq!(a.len(), 1);
    /// ```
    #[rune::function]
    fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns the number of elements the map can hold without reallocating.
    ///
    /// This number is a lower bound; the `HashMap<K, V>` might be able to hold
    /// more, but is guaranteed to be able to hold at least this many.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    /// let map = HashMap::with_capacity(100);
    /// assert!(map.capacity() >= 100);
    /// ```
    #[rune::function]
    fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let a = HashMap::new();
    /// assert!(a.is_empty());
    /// a.insert(1, "a");
    /// assert!(!a.is_empty());
    /// ```
    #[rune::function]
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// let pairs = map.iter().collect::<Vec>();
    /// pairs.sort();
    /// assert_eq!(pairs, [("a", 1), ("b", 2), ("c", 3)]);
    /// ```
    ///
    /// # Performance
    ///
    /// In the current implementation, iterating over map takes O(capacity) time
    /// instead of O(len) because it internally visits empty buckets too.
    #[rune::function]
    fn iter(&self) -> Iterator {
        let iter = self.map.clone().into_iter();
        Iterator::from("std::collections::map::Iter", iter)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical. See the [module-level
    /// documentation] for more.
    ///
    /// [module-level documentation]: crate::collections#insert-and-complex-keys
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Some("b"));
    /// assert_eq!(map[37], "c");
    /// ```
    #[rune::function]
    fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    /// Returns the value corresponding to the [`Key`].
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get(1), Some("a"));
    /// assert_eq!(map.get(2), None);
    /// ```
    #[rune::function]
    fn get(&self, key: Key) -> Option<Value> {
        self.map.get(&key).cloned()
    }

    /// Returns `true` if the map contains a value for the specified [`Key`].
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.contains_key(1), true);
    /// assert_eq!(map.contains_key(2), false);
    /// ```
    #[rune::function]
    fn contains_key(&self, key: Key) -> bool {
        self.map.contains_key(&key)
    }

    /// Removes a key from the map, returning the value at the [`Key`] if the
    /// key was previously in the map.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(1), Some("a"));
    /// assert_eq!(map.remove(1), None);
    /// ```
    #[rune::function]
    fn remove(&mut self, key: Key) -> Option<Value> {
        self.map.remove(&key)
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let a = HashMap::new();
    /// a.insert(1, "a");
    /// a.clear();
    /// assert!(a.is_empty());
    /// ```
    #[rune::function]
    fn clear(&mut self) {
        self.map.clear()
    }

    /// An iterator visiting all keys in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// let keys = map.keys().collect::<Vec>();
    /// keys.sort();
    /// assert_eq!(keys, ["a", "b", "c"]);
    /// ```
    ///
    /// # Performance
    ///
    /// In the current implementation, iterating over keys takes O(capacity)
    /// time instead of O(len) because it internally visits empty buckets too.
    #[rune::function]
    fn keys(&self) -> Iterator {
        let iter = self.map.keys().cloned().collect::<Vec<_>>().into_iter();
        Iterator::from("std::collections::map::Keys", iter)
    }

    /// An iterator visiting all values in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// let values = map.values().collect::<Vec>();
    /// values.sort();
    /// assert_eq!(values, [1, 2, 3]);
    /// ```
    ///
    /// # Performance
    ///
    /// In the current implementation, iterating over values takes O(capacity)
    /// time instead of O(len) because it internally visits empty buckets too.
    #[rune::function]
    fn values(&self) -> Iterator {
        let iter = self.map.values().cloned().collect::<Vec<_>>().into_iter();
        Iterator::from("std::collections::map::Values", iter)
    }

    /// Extend this map from an iterator.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    ///
    /// map.extend([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    /// ```
    #[rune::function]
    fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            let (key, value) = vm_try!(<(Key, Value)>::from_value(value));
            self.map.insert(key, value);
        }

        VmResult::Ok(())
    }

    pub(crate) fn from_iter(mut it: Iterator) -> VmResult<Self> {
        let mut map = collections::HashMap::new();

        while let Some(value) = vm_try!(it.next()) {
            let (key, value) = vm_try!(<(Key, Value)>::from_value(value));
            map.insert(key, value);
        }

        VmResult::Ok(Self { map })
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// let pairs = [];
    ///
    /// for pair in map {
    ///     pairs.push(pair);
    /// }
    ///
    /// pairs.sort();
    /// assert_eq!(pairs, [("a", 1), ("b", 2), ("c", 3)]);
    /// ```
    ///
    /// # Performance
    ///
    /// In the current implementation, iterating over map takes O(capacity) time
    /// instead of O(len) because it internally visits empty buckets too.
    #[rune::function(protocol = INTO_ITER)]
    fn into_iter(&self) -> Iterator {
        self.__rune_fn__iter()
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did have this key present, the value is updated.
    ///
    /// [module-level documentation]: crate::collections#insert-and-complex-keys
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// map[37] = "a";
    /// assert!(!map.is_empty());
    ///
    /// map[37] = "c";
    /// assert_eq!(map[37], "c");
    /// ```
    #[rune::function(protocol = INDEX_SET)]
    fn index_set(&mut self, key: Key, value: Value) {
        let _ = self.map.insert(key, value);
    }

    /// Returns a the value corresponding to the key.
    ///
    /// # Panics
    ///
    /// Panics if the given value is not present in the map.
    ///
    /// ```rune,should_panic
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// let _ = map[1];
    /// ```
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// map[1] = "a";
    /// assert_eq!(map[1], "a");
    /// ```
    #[rune::function(protocol = INDEX_GET)]
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

    /// Debug format the current map.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// map[1] = "a";
    ///
    /// assert_eq!(format!("{:?}", map), "{1: \"a\"}");
    /// ```
    #[rune::function(protocol = STRING_DEBUG)]
    fn string_debug(&self, s: &mut String) -> VmResult<fmt::Result> {
        self.string_debug_with(s, &mut EnvProtocolCaller)
    }

    pub(crate) fn string_debug_with(
        &self,
        s: &mut String,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<fmt::Result> {
        vm_write!(s, "{{");

        let mut it = self.map.iter().peekable();

        while let Some((key, value)) = it.next() {
            vm_write!(s, "{:?}: ", key);

            if let Err(fmt::Error) = vm_try!(value.string_debug_with(s, caller)) {
                return VmResult::Ok(Err(fmt::Error));
            }

            if it.peek().is_some() {
                vm_write!(s, ", ");
            }
        }

        vm_write!(s, "}}");
        VmResult::Ok(Ok(()))
    }

    /// Perform a partial equality check over two maps.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    ///
    /// let map1 = HashMap::from([
    ///     ("a", 1.0),
    ///     ("c", 3.0),
    ///     ("b", 2.0),
    /// ]);
    ///
    /// let map2 = HashMap::from([
    ///     ("c", 3.0),
    ///     ("a", 1.0),
    ///     ("b", 2.0),
    /// ]);
    ///
    /// assert!(map1 == map2);
    ///
    /// map1["b"] = f64::NAN;
    /// map2["b"] = f64::NAN;
    ///
    /// assert!(map1 != map2);
    /// ```
    #[rune::function(protocol = PARTIAL_EQ)]
    fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        self.partial_eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn partial_eq_with(
        &self,
        other: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        if self.map.len() != other.map.len() {
            return VmResult::Ok(false);
        }

        for (k, v) in self.map.iter() {
            let Some(v2) = other.map.get(k) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(Value::partial_eq_with(v, v2, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    /// Perform a total equality check over two maps.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashMap;
    /// use std::ops::eq;
    ///
    /// let map1 = HashMap::from([
    ///     ("a", 1),
    ///     ("c", 3),
    ///     ("b", 2),
    /// ]);
    ///
    /// let map2 = HashMap::from([
    ///     ("c", 3),
    ///     ("a", 1),
    ///     ("b", 2),
    /// ]);
    ///
    /// assert!(eq(map1, map2));
    /// ```
    #[rune::function(protocol = EQ)]
    fn eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    fn eq_with(&self, other: &Self, caller: &mut EnvProtocolCaller) -> VmResult<bool> {
        if self.map.len() != other.map.len() {
            return VmResult::Ok(false);
        }

        for (k, v) in self.map.iter() {
            let Some(v2) = other.map.get(k) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(Value::eq_with(v, v2, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }
}

/// Convert a hashmap from a `value`.
///
/// The hashmap can be converted from anything that implements the [`INTO_ITER`]
/// protocol, and each item produces should be a tuple pair.
#[rune::function(path = HashMap::from)]
fn from(value: Value) -> VmResult<HashMap> {
    HashMap::from_iter(vm_try!(value.into_iter()))
}

/// Clone the map.
///
/// # Examples
///
/// ```rune
/// use std::collections::HashMap;
///
/// let a = HashMap::from([("a", 1), ("b", 2)]);
/// let b = a.clone();
///
/// b.insert("c", 3);
///
/// assert_eq!(a.len(), 2);
/// assert_eq!(b.len(), 3);
/// ```
#[rune::function(instance)]
fn clone(this: &HashMap) -> HashMap {
    this.clone()
}
