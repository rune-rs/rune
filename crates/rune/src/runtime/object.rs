use core::borrow;
use core::cmp;
use core::cmp::Ordering;
use core::fmt;
use core::hash;
use core::iter;

use crate as rune;
use crate::alloc::hashbrown::raw::RawIter;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::alloc::{hash_map, HashMap};
use crate::runtime::{FromValue, ProtocolCaller, RawRef, Ref, ToValue, Value, VmResult};
use crate::{Any, ItemBuf};

/// An owning iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`into_iter`] method on [`Object`]
/// (provided by the `IntoIterator` trait). See its documentation for more.
///
/// [`into_iter`]: struct.Object.html#method.into_iter
/// [`Object`]: struct.Object.html
pub type IntoIter = hash_map::IntoIter<String, Value>;

/// A mutable iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`iter_mut`] method on [`Object`]. See its
/// documentation for more.
///
/// [`iter_mut`]: struct.Object.html#method.iter_mut
/// [`Object`]: struct.Object.html
pub type IterMut<'a> = hash_map::IterMut<'a, String, Value>;

/// An iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`iter`] method on [`Object`]. See its
/// documentation for more.
///
/// [`iter`]: struct.Object.html#method.iter
/// [`Object`]: struct.Object.html
pub type Iter<'a> = hash_map::Iter<'a, String, Value>;

/// An iterator over the keys of a `HashMap`.
///
/// This `struct` is created by the [`keys`] method on [`Object`]. See its
/// documentation for more.
///
/// [`keys`]: struct.Object.html#method.keys
/// [`Object`]: struct.Object.html
pub type Keys<'a> = hash_map::Keys<'a, String, Value>;

/// An iterator over the values of a `HashMap`.
///
/// This `struct` is created by the [`values`] method on [`Object`]. See its
/// documentation for more.
///
/// [`values`]: struct.Object.html#method.values
/// [`Object`]: struct.Object.html
pub type Values<'a> = hash_map::Values<'a, String, Value>;

/// Struct representing a dynamic anonymous object.
///
/// # Rust Examples
///
/// ```rust
/// use rune::alloc::String;
///
/// let mut object = rune::runtime::Object::new();
/// assert!(object.is_empty());
///
/// object.insert_value(String::try_from("foo")?, 42).into_result()?;
/// object.insert_value(String::try_from("bar")?, true).into_result()?;
/// assert_eq!(2, object.len());
///
/// assert_eq!(Some(42), object.get_value("foo").into_result()?);
/// assert_eq!(Some(true), object.get_value("bar").into_result()?);
/// assert_eq!(None::<bool>, object.get_value("baz").into_result()?);
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Any, Default)]
#[repr(transparent)]
#[rune(builtin, static_type = OBJECT)]
pub struct Object {
    inner: HashMap<String, Value>,
}

impl Object {
    /// Construct a new object.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = Object::new();
    /// object.insert("Hello", "World");
    /// ```
    #[inline]
    #[rune::function(keep, path = Self::new)]
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Construct a new object with the given capacity.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = Object::with_capacity(16);
    /// object.insert("Hello", "World");
    /// ```
    #[inline]
    #[rune::function(path = Self::with_capacity)]
    pub(crate) fn rune_with_capacity(capacity: usize) -> VmResult<Self> {
        VmResult::Ok(vm_try!(Self::with_capacity(capacity)))
    }

    /// Construct a new object with the given capacity.
    pub fn with_capacity(capacity: usize) -> alloc::Result<Self> {
        // BTreeMap doesn't support setting capacity on creation but we keep
        // this here in case we want to switch store later.
        Ok(Self {
            inner: HashMap::try_with_capacity(capacity)?,
        })
    }

    /// Returns the number of elements in the object.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = Object::with_capacity(16);
    /// object.insert("Hello", "World");
    /// assert_eq!(object.len(), 1);
    /// ```
    #[inline]
    #[rune::function(keep)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the object is empty.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = Object::with_capacity(16);
    /// assert!(object.is_empty());
    /// object.insert("Hello", "World");
    /// assert!(!object.is_empty());
    /// ```
    #[inline]
    #[rune::function(keep)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q>(&self, k: &Q) -> Option<&Value>
    where
        String: borrow::Borrow<Q>,
        Q: ?Sized + hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.get(k)
    }

    /// Get the given value at the given index.
    pub fn get_value<Q, T>(&self, k: &Q) -> VmResult<Option<T>>
    where
        String: borrow::Borrow<Q>,
        Q: ?Sized + hash::Hash + cmp::Eq + cmp::Ord,
        T: FromValue,
    {
        let value = match self.inner.get(k) {
            Some(value) => value.clone(),
            None => return VmResult::Ok(None),
        };

        VmResult::Ok(Some(vm_try!(T::from_value(value))))
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: borrow::Borrow<Q>,
        Q: ?Sized + hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.get_mut(k)
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        String: borrow::Borrow<Q>,
        Q: ?Sized + hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.contains_key(k)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    #[inline]
    pub fn remove<Q>(&mut self, k: &Q) -> Option<Value>
    where
        String: borrow::Borrow<Q>,
        Q: ?Sized + hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.remove(k)
    }

    /// Inserts a key-value pair into the dynamic object, converting it as
    /// necessary through the [`ToValue`] trait.
    #[inline]
    pub fn insert_value<T>(&mut self, k: String, v: T) -> VmResult<()>
    where
        T: ToValue,
    {
        vm_try!(self.inner.try_insert(k, vm_try!(v.to_value())));
        VmResult::Ok(())
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let map = #{};
    /// assert_eq!(map.insert("a", 1), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert("b", 2);
    /// assert_eq!(map.insert("b", 3), Some(2));
    /// assert_eq!(map["b"], 3);
    /// ```
    #[inline]
    #[rune::function(path = Self::insert)]
    pub(crate) fn rune_insert(&mut self, k: String, v: Value) -> VmResult<Option<Value>> {
        VmResult::Ok(vm_try!(self.inner.try_insert(k, v)))
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    pub fn insert(&mut self, k: String, v: Value) -> alloc::Result<Option<Value>> {
        self.inner.try_insert(k, v)
    }

    /// Clears the object, removing all key-value pairs. Keeps the allocated
    /// memory for reuse.
    #[inline]
    #[rune::function(keep)]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Convert into inner.
    pub fn into_inner(self) -> HashMap<String, Value> {
        self.inner
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    /// The iterator element type is `(&'a String, &'a Value)`.
    pub fn iter(&self) -> Iter<'_> {
        self.inner.iter()
    }

    /// An iterator visiting all keys in arbitrary order.
    /// The iterator element type is `&'a String`.
    pub fn keys(&self) -> Keys<'_> {
        self.inner.keys()
    }

    /// An iterator visiting all values in arbitrary order.
    /// The iterator element type is `&'a Value`.
    pub fn values(&self) -> Values<'_> {
        self.inner.values()
    }

    /// An iterator visiting all key-value pairs in arbitrary order,
    /// with mutable references to the values.
    ///
    /// The iterator element type is `(&'a String, &'a mut Value)`.
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        self.inner.iter_mut()
    }

    /// An iterator visiting all keys and values in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = #{a: 1, b: 2, c: 3};
    /// let vec = [];
    ///
    /// for key in object.iter() {
    ///     vec.push(key);
    /// }
    ///
    /// vec.sort_by(|a, b| a.0.cmp(b.0));
    /// assert_eq!(vec, [("a", 1), ("b", 2), ("c", 3)]);
    /// ```
    #[rune::function(keep, path = Self::iter)]
    pub fn rune_iter(this: Ref<Self>) -> RuneIter {
        // SAFETY: we're holding onto the related reference guard, and making
        // sure that it's dropped after the iterator.
        let iter = unsafe { this.inner.raw_table().iter() };
        let (_, _guard) = Ref::into_raw(this);
        RuneIter { iter, _guard }
    }

    /// An iterator visiting all keys in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = #{a: 1, b: 2, c: 3};
    /// let vec = [];
    ///
    /// for key in object.keys() {
    ///     vec.push(key);
    /// }
    ///
    /// vec.sort_by(|a, b| a.cmp(b));
    /// assert_eq!(vec, ["a", "b", "c"]);
    /// ```
    #[rune::function(keep, path = Self::keys)]
    pub fn rune_keys(this: Ref<Self>) -> RuneIterKeys {
        // SAFETY: we're holding onto the related reference guard, and making
        // sure that it's dropped after the iterator.
        let iter = unsafe { this.inner.raw_table().iter() };
        let (_, _guard) = Ref::into_raw(this);
        RuneIterKeys { iter, _guard }
    }

    /// An iterator visiting all values in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let object = #{a: 1, b: 2, c: 3};
    /// let vec = [];
    ///
    /// for key in object.values() {
    ///     vec.push(key);
    /// }
    ///
    /// vec.sort_by(|a, b| a.cmp(b));
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[rune::function(keep, path = Self::values)]
    pub fn rune_values(this: Ref<Self>) -> RuneValues {
        // SAFETY: we're holding onto the related reference guard, and making
        // sure that it's dropped after the iterator.
        let iter = unsafe { this.inner.raw_table().iter() };
        let (_, _guard) = Ref::into_raw(this);
        RuneValues { iter, _guard }
    }

    pub(crate) fn partial_eq_with(
        a: &Self,
        b: Value,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        let mut b = vm_try!(b.into_iter());

        for (k1, v1) in a.iter() {
            let Some(value) = vm_try!(b.next()) else {
                return VmResult::Ok(false);
            };

            let (k2, v2) = vm_try!(<(Ref<String>, Value)>::from_value(value));

            if k1 != &*k2 {
                return VmResult::Ok(false);
            }

            if !vm_try!(Value::partial_eq_with(v1, &v2, caller)) {
                return VmResult::Ok(false);
            }
        }

        if vm_try!(b.next()).is_some() {
            return VmResult::Ok(false);
        }

        VmResult::Ok(true)
    }

    pub(crate) fn eq_with(
        a: &Self,
        b: &Self,
        eq: fn(&Value, &Value, &mut dyn ProtocolCaller) -> VmResult<bool>,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        if a.inner.len() != b.inner.len() {
            return VmResult::Ok(false);
        }

        for (key, a) in a.inner.iter() {
            let Some(b) = b.inner.get(key) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(eq(a, b, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    pub(crate) fn partial_cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        let mut b = b.inner.iter();

        for (k1, v1) in a.inner.iter() {
            let Some((k2, v2)) = b.next() else {
                return VmResult::Ok(Some(Ordering::Greater));
            };

            match k1.partial_cmp(k2) {
                Some(Ordering::Equal) => (),
                other => return VmResult::Ok(other),
            }

            match Value::partial_cmp_with(v1, v2, caller) {
                VmResult::Ok(Some(Ordering::Equal)) => (),
                other => return other,
            }
        }

        if b.next().is_some() {
            return VmResult::Ok(Some(Ordering::Less));
        }

        VmResult::Ok(Some(Ordering::Equal))
    }

    pub(crate) fn cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<Ordering> {
        let mut b = b.inner.iter();

        for (k1, v1) in a.inner.iter() {
            let Some((k2, v2)) = b.next() else {
                return VmResult::Ok(Ordering::Greater);
            };

            match k1.cmp(k2) {
                Ordering::Equal => (),
                other => return VmResult::Ok(other),
            }

            match Value::cmp_with(v1, v2, caller) {
                VmResult::Ok(Ordering::Equal) => (),
                other => return other,
            }
        }

        if b.next().is_some() {
            return VmResult::Ok(Ordering::Less);
        }

        VmResult::Ok(Ordering::Equal)
    }

    /// Debug implementation for a struct. This assumes that all fields
    /// corresponds to identifiers.
    pub(crate) fn debug_struct<'a>(&'a self, item: &'a ItemBuf) -> DebugStruct<'a> {
        DebugStruct { item, st: self }
    }
}

impl TryClone for Object {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
        })
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = (&'a String, &'a Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Object {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl IntoIterator for Object {
    type Item = (String, Value);
    type IntoIter = IntoIter;

    /// Creates a consuming iterator, that is, one that moves each key-value
    /// pair out of the object in arbitrary order. The object cannot be used
    /// after calling this.
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.inner.iter()).finish()
    }
}

from_value2!(Object, into_object_ref, into_object_mut, into_object);

pub struct DebugStruct<'a> {
    item: &'a ItemBuf,
    st: &'a Object,
}

impl fmt::Display for DebugStruct<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ::rust_alloc::string::ToString;

        let mut d = f.debug_struct(&self.item.to_string());

        for (key, value) in self.st.iter() {
            d.field(key, value);
        }

        d.finish()
    }
}

#[derive(Any)]
#[rune(item = ::std::object, name = Iter)]
pub struct RuneIter {
    iter: RawIter<(String, Value)>,
    _guard: RawRef,
}

impl RuneIter {
    #[rune::function(instance, keep, protocol = NEXT)]
    pub fn next(&mut self) -> Option<VmResult<(String, Value)>> {
        unsafe {
            let bucket = self.iter.next()?;
            let (key, value) = bucket.as_ref();

            let key = match key.try_clone() {
                Ok(key) => key,
                Err(err) => return Some(VmResult::err(err)),
            };

            Some(VmResult::Ok((key, value.clone())))
        }
    }

    #[rune::function(instance, keep, protocol = SIZE_HINT)]
    pub fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[rune::function(instance, keep, protocol = LEN)]
    pub fn len(&self) -> usize {
        self.iter.len()
    }
}

impl iter::Iterator for RuneIter {
    type Item = VmResult<(String, Value)>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        RuneIter::next(self)
    }
}

#[derive(Any)]
#[rune(item = ::std::object, name = Keys)]
pub struct RuneIterKeys {
    iter: RawIter<(String, Value)>,
    _guard: RawRef,
}

impl RuneIterKeys {
    #[rune::function(instance, keep, protocol = NEXT)]
    pub fn next(&mut self) -> Option<VmResult<String>> {
        unsafe {
            let bucket = self.iter.next()?;
            let (key, _) = bucket.as_ref();

            let key = match key.try_clone() {
                Ok(key) => key,
                Err(err) => return Some(VmResult::err(err)),
            };

            Some(VmResult::Ok(key))
        }
    }

    #[rune::function(instance, keep, protocol = SIZE_HINT)]
    pub fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[rune::function(instance, keep, protocol = LEN)]
    pub fn len(&self) -> usize {
        self.iter.len()
    }
}

impl iter::Iterator for RuneIterKeys {
    type Item = VmResult<String>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        RuneIterKeys::next(self)
    }
}

#[derive(Any)]
#[rune(item = ::std::object, name = Values)]
pub struct RuneValues {
    iter: RawIter<(String, Value)>,
    _guard: RawRef,
}

impl RuneValues {
    #[rune::function(instance, keep, protocol = NEXT)]
    pub fn next(&mut self) -> Option<VmResult<Value>> {
        unsafe {
            let bucket = self.iter.next()?;
            let (_, value) = bucket.as_ref();
            Some(VmResult::Ok(value.clone()))
        }
    }

    #[rune::function(instance, keep, protocol = SIZE_HINT)]
    pub fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[rune::function(instance, keep, protocol = LEN)]
    pub fn len(&self) -> usize {
        self.iter.len()
    }
}

impl iter::Iterator for RuneValues {
    type Item = VmResult<Value>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        RuneValues::next(self)
    }
}
