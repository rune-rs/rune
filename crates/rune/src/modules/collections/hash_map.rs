use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::hashbrown::Table;
use crate::runtime::{
    EnvProtocolCaller, Formatter, FromValue, Iterator, ProtocolCaller, Ref, Value, VmErrorKind,
    VmResult,
};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<HashMap>()?;
    module.function_meta(HashMap::new__meta)?;
    module.function_meta(HashMap::with_capacity__meta)?;
    module.function_meta(HashMap::len__meta)?;
    module.function_meta(HashMap::capacity__meta)?;
    module.function_meta(HashMap::insert__meta)?;
    module.function_meta(HashMap::get__meta)?;
    module.function_meta(HashMap::contains_key__meta)?;
    module.function_meta(HashMap::remove__meta)?;
    module.function_meta(HashMap::clear__meta)?;
    module.function_meta(HashMap::is_empty__meta)?;
    module.function_meta(HashMap::iter__meta)?;
    module.function_meta(HashMap::keys__meta)?;
    module.function_meta(HashMap::values__meta)?;
    module.function_meta(HashMap::extend__meta)?;
    module.function_meta(HashMap::from__meta)?;
    module.function_meta(HashMap::clone__meta)?;
    module.function_meta(HashMap::index_set__meta)?;
    module.function_meta(HashMap::index_get__meta)?;
    module.function_meta(HashMap::string_debug__meta)?;
    module.function_meta(HashMap::partial_eq__meta)?;
    module.function_meta(HashMap::eq__meta)?;
    module.function_meta(HashMap::into_iter__meta)?;
    Ok(())
}

#[derive(Any)]
#[rune(item = ::std::collections)]
pub(crate) struct HashMap {
    table: Table<Value>,
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
    #[rune::function(keep, path = Self::new)]
    fn new() -> Self {
        Self {
            table: Table::new(),
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
    #[rune::function(keep, path = Self::with_capacity)]
    fn with_capacity(capacity: usize) -> VmResult<Self> {
        VmResult::Ok(Self {
            table: vm_try!(Table::try_with_capacity(capacity)),
        })
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
    #[rune::function(keep)]
    fn len(&self) -> usize {
        self.table.len()
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
    #[rune::function(keep)]
    fn capacity(&self) -> usize {
        self.table.capacity()
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
    #[rune::function(keep)]
    fn is_empty(&self) -> bool {
        self.table.is_empty()
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
    #[rune::function(keep)]
    fn insert(&mut self, key: Value, value: Value) -> VmResult<Option<Value>> {
        let mut caller = EnvProtocolCaller;
        self.table.insert_with(key, value, &mut caller)
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
    #[rune::function(keep)]
    fn get(&self, key: Value) -> VmResult<Option<Value>> {
        let mut caller = EnvProtocolCaller;
        VmResult::Ok(vm_try!(self.table.get(&key, &mut caller)).map(|(_, v)| v.clone()))
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
    #[rune::function(keep)]
    fn contains_key(&self, key: Value) -> VmResult<bool> {
        let mut caller = EnvProtocolCaller;
        VmResult::Ok(vm_try!(self.table.get(&key, &mut caller)).is_some())
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
    #[rune::function(keep)]
    fn remove(&mut self, key: Value) -> VmResult<Option<Value>> {
        let mut caller = EnvProtocolCaller;
        self.table.remove_with(&key, &mut caller)
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
    #[rune::function(keep)]
    fn clear(&mut self) {
        self.table.clear()
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
    #[rune::function(keep, instance, path = Self::iter)]
    fn iter(this: Ref<Self>) -> Iterator {
        let iter = Table::iter_ref(Ref::map(this, |this| &this.table));
        Iterator::from("std::collections::hash_map::Iter", iter)
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
    #[rune::function(keep, instance, path = Self::keys)]
    fn keys(this: Ref<Self>) -> Iterator {
        let iter = Table::keys_ref(Ref::map(this, |this| &this.table));
        Iterator::from("std::collections::hash_map::Keys", iter)
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
    #[rune::function(keep, instance, path = Self::values)]
    fn values(this: Ref<Self>) -> Iterator {
        let iter = Table::values_ref(Ref::map(this, |this| &this.table));

        Iterator::from("std::collections::hash_map::Values", iter)
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
    #[rune::function(keep)]
    fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            let (key, value) = vm_try!(<(Value, Value)>::from_value(value));
            vm_try!(self.insert(key, value));
        }

        VmResult::Ok(())
    }

    /// Convert a hashmap from a `value`.
    ///
    /// The hashmap can be converted from anything that implements the [`INTO_ITER`]
    /// protocol, and each item produces should be a tuple pair.
    #[rune::function(keep, path = Self::from)]
    fn from(value: Value) -> VmResult<HashMap> {
        let mut caller = EnvProtocolCaller;
        HashMap::from_iter(vm_try!(value.into_iter()), &mut caller)
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
    #[rune::function(keep, instance, path = Self::clone)]
    fn clone(this: &HashMap) -> VmResult<HashMap> {
        VmResult::Ok(Self {
            table: vm_try!(this.table.try_clone()),
        })
    }

    pub(crate) fn from_iter<P>(mut it: Iterator, caller: &mut P) -> VmResult<Self>
    where
        P: ?Sized + ProtocolCaller,
    {
        let mut map = Self::new();

        while let Some(value) = vm_try!(it.next()) {
            let (key, value) = vm_try!(<(Value, Value)>::from_value(value));
            vm_try!(map.table.insert_with(key, value, caller));
        }

        VmResult::Ok(map)
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
    #[rune::function(keep, protocol = INDEX_SET)]
    fn index_set(&mut self, key: Value, value: Value) -> VmResult<()> {
        let _ = vm_try!(self.insert(key, value));
        VmResult::Ok(())
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
    #[rune::function(keep, protocol = INDEX_GET)]
    fn index_get(&self, key: Value) -> VmResult<Value> {
        use crate::runtime::TypeOf;

        let mut caller = EnvProtocolCaller;

        let Some((_, value)) = vm_try!(self.table.get(&key, &mut caller)) else {
            return VmResult::err(VmErrorKind::MissingIndexKey {
                target: Self::type_info(),
            });
        };

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
    #[rune::function(keep, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        self.string_debug_with(f, &mut EnvProtocolCaller)
    }

    pub(crate) fn string_debug_with(
        &self,
        f: &mut Formatter,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        vm_write!(f, "{{");

        let mut it = self.table.iter().peekable();

        while let Some((key, value)) = it.next() {
            vm_try!(key.string_debug_with(f, caller));
            vm_write!(f, ": ");
            vm_try!(value.string_debug_with(f, caller));

            if it.peek().is_some() {
                vm_write!(f, ", ");
            }
        }

        vm_write!(f, "}}");
        VmResult::Ok(())
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
    #[rune::function(keep, protocol = PARTIAL_EQ)]
    fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        self.partial_eq_with(other, &mut EnvProtocolCaller)
    }

    fn partial_eq_with(&self, other: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if self.table.len() != other.table.len() {
            return VmResult::Ok(false);
        }

        for (k, v1) in self.table.iter() {
            let Some((_, v2)) = vm_try!(other.table.get(k, caller)) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(Value::partial_eq_with(v1, v2, caller)) {
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
    #[rune::function(keep, protocol = EQ)]
    fn eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    fn eq_with(&self, other: &Self, caller: &mut EnvProtocolCaller) -> VmResult<bool> {
        if self.table.len() != other.table.len() {
            return VmResult::Ok(false);
        }

        for (k, v1) in self.table.iter() {
            let Some((_, v2)) = vm_try!(other.table.get(k, caller)) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(Value::eq_with(v1, v2, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
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
    #[rune::function(keep, instance, protocol = INTO_ITER, path = Self)]
    fn into_iter(this: Ref<Self>) -> Iterator {
        Self::iter(this)
    }
}
