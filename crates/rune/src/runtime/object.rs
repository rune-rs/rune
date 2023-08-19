use core::borrow;
use core::cmp;
use core::cmp::Ordering;
use core::fmt;
use core::hash;
use core::iter;

use crate::no_std::collections::{btree_map, BTreeMap};
use crate::no_std::prelude::*;

use crate as rune;
use crate::compile::{ItemBuf, Named};
use crate::module::InstallWith;
use crate::runtime::{FromValue, Iterator, ProtocolCaller, RawStr, ToValue, Value, VmResult};

/// An owning iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`into_iter`] method on [`Object`]
/// (provided by the `IntoIterator` trait). See its documentation for more.
///
/// [`into_iter`]: struct.Object.html#method.into_iter
/// [`Object`]: struct.Object.html
pub type IntoIter = btree_map::IntoIter<String, Value>;

/// A mutable iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`iter_mut`] method on [`Object`]. See its
/// documentation for more.
///
/// [`iter_mut`]: struct.Object.html#method.iter_mut
/// [`Object`]: struct.Object.html
pub type IterMut<'a> = btree_map::IterMut<'a, String, Value>;

/// An iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`iter`] method on [`Object`]. See its
/// documentation for more.
///
/// [`iter`]: struct.Object.html#method.iter
/// [`Object`]: struct.Object.html
pub type Iter<'a> = btree_map::Iter<'a, String, Value>;

/// An iterator over the keys of a `HashMap`.
///
/// This `struct` is created by the [`keys`] method on [`Object`]. See its
/// documentation for more.
///
/// [`keys`]: struct.Object.html#method.keys
/// [`Object`]: struct.Object.html
pub type Keys<'a> = btree_map::Keys<'a, String, Value>;

/// An iterator over the values of a `HashMap`.
///
/// This `struct` is created by the [`values`] method on [`Object`]. See its
/// documentation for more.
///
/// [`values`]: struct.Object.html#method.values
/// [`Object`]: struct.Object.html
pub type Values<'a> = btree_map::Values<'a, String, Value>;

/// Struct representing a dynamic anonymous object.
///
/// # Examples
///
/// ```
/// let mut object = rune::runtime::Object::new();
/// assert!(object.is_empty());
///
/// object.insert_value(String::from("foo"), 42).into_result()?;
/// object.insert_value(String::from("bar"), true).into_result()?;
/// assert_eq!(2, object.len());
///
/// assert_eq!(Some(42), object.get_value("foo").into_result()?);
/// assert_eq!(Some(true), object.get_value("bar").into_result()?);
/// assert_eq!(None::<bool>, object.get_value("baz").into_result()?);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Default, Clone)]
#[repr(transparent)]
pub struct Object {
    inner: BTreeMap<String, Value>,
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
            inner: BTreeMap::new(),
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
    #[rune::function(keep, path = Self::with_capacity)]
    pub fn with_capacity(#[allow(unused)] capacity: usize) -> Self {
        // BTreeMap doesn't support setting capacity on creation but we keep
        // this here in case we want to switch store later.
        Self {
            inner: BTreeMap::new(),
        }
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
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Value>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.get(k)
    }

    /// Get the given value at the given index.
    pub fn get_value<Q: ?Sized, T>(&self, k: &Q) -> VmResult<Option<T>>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq + cmp::Ord,
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
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.get_mut(k)
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq + cmp::Ord,
    {
        self.inner.contains_key(k)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<Value>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq + cmp::Ord,
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
        self.inner.insert(k, vm_try!(v.to_value()));
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
    #[rune::function(keep)]
    pub fn insert(&mut self, k: String, v: Value) -> Option<Value> {
        self.inner.insert(k, v)
    }

    /// Clears the object, removing all key-value pairs. Keeps the allocated
    /// memory for reuse.
    #[inline]
    #[rune::function(keep)]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Convert into inner.
    pub fn into_inner(self) -> BTreeMap<String, Value> {
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
    pub fn rune_iter(&self) -> Iterator {
        Iterator::from("std::object::Iter", self.clone().into_iter())
    }

    pub(crate) fn partial_eq_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        if a.inner.len() != b.inner.len() {
            return VmResult::Ok(false);
        }

        for (key, a) in a.inner.iter() {
            let Some(b) = b.inner.get(key) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(Value::partial_eq_with(a, b, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if a.inner.len() != b.inner.len() {
            return VmResult::Ok(false);
        }

        for (key, a) in a.inner.iter() {
            let Some(b) = b.inner.get(key) else {
                return VmResult::Ok(false);
            };

            if !vm_try!(Value::eq_with(a, b, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    pub(crate) fn cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
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

impl iter::FromIterator<(String, Value)> for Object {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(src: T) -> Self {
        Self {
            inner: src.into_iter().collect(),
        }
    }
}

from_value!(Object, into_object);

impl Named for Object {
    const BASE_NAME: RawStr = RawStr::from_str("Object");
}

impl InstallWith for Object {}

pub struct DebugStruct<'a> {
    item: &'a ItemBuf,
    st: &'a Object,
}

impl fmt::Display for DebugStruct<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct(&self.item.to_string());

        for (key, value) in self.st.iter() {
            d.field(key, value);
        }

        d.finish()
    }
}
