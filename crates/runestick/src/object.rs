use crate::collections::{btree_map, BTreeMap};
use crate::{
    FromValue, InstallWith, Item, Mut, Named, RawMut, RawRef, RawStr, Ref, ToValue,
    UnsafeFromValue, Value, Vm, VmError,
};
use std::borrow;
use std::cmp;
use std::fmt;
use std::hash;
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
/// ```rust
/// # fn main() -> runestick::Result<()> {
/// let mut object = runestick::Object::new();
/// assert!(object.is_empty());
///
/// object.insert_value(String::from("foo"), 42)?;
/// object.insert_value(String::from("bar"), true)?;
/// assert_eq!(2, object.len());
///
/// assert_eq!(Some(42), object.get_value("foo")?);
/// assert_eq!(Some(true), object.get_value("bar")?);
/// assert_eq!(None::<bool>, object.get_value("baz")?);
/// # Ok(()) }
/// ```
#[derive(Default, Clone)]
#[repr(transparent)]
pub struct Object {
    inner: BTreeMap<String, Value>,
}

impl Object {
    /// Construct a new object.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// Construct a new object with the given capacity.
    #[inline]
    pub fn with_capacity(_cap: usize) -> Self {
        /* BTreeMap doesn't support setting capacity on creation but we keep this here in case we want to switch store later */
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// Returns the number of elements in the object.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the Object contains no elements.
    #[inline]
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
    pub fn get_value<Q: ?Sized, T>(&self, k: &Q) -> Result<Option<T>, VmError>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq + cmp::Ord,
        T: FromValue,
    {
        let value = match self.inner.get(k) {
            Some(value) => value.clone(),
            None => return Ok(None),
        };

        Ok(Some(T::from_value(value)?))
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

    /// Removes a key from the object, returning the value at the key if the key
    /// was previously in the object.
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
    pub fn insert_value<T>(&mut self, k: String, v: T) -> Result<(), VmError>
    where
        T: ToValue,
    {
        self.inner.insert(k, v.to_value()?);
        Ok(())
    }

    /// Inserts a key-value pair into the dynamic object.
    #[inline]
    pub fn insert(&mut self, k: String, v: Value) -> Option<Value> {
        self.inner.insert(k, v)
    }

    /// Clears the object, removing all key-value pairs. Keeps the allocated
    /// memory for reuse.
    #[inline]
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
    /// The iterator element type is `(&'a String, &'a mut Value)`.
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        self.inner.iter_mut()
    }

    /// Value pointer equals implementation for an Object.
    pub(crate) fn value_ptr_eq(vm: &mut Vm, a: &Self, b: &Self) -> Result<bool, VmError> {
        map_ptr_eq(vm, &a.inner, &b.inner)
    }

    /// Debug implementation for a struct. This assumes that all fields
    /// corresponds to identifiers.
    pub(crate) fn debug_struct<'a>(&'a self, item: &'a Item) -> DebugStruct<'a> {
        DebugStruct { item, st: self }
    }

    /// Convert into a runestick iterator.
    pub fn into_iterator(&self) -> crate::Iterator {
        crate::Iterator::from("std::object::Iter", self.clone().into_iter())
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

impl std::iter::FromIterator<(String, Value)> for Object {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(src: T) -> Self {
        Self {
            inner: src.into_iter().collect(),
        }
    }
}

impl FromValue for Object {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_object()?.take()?)
    }
}

impl FromValue for Mut<Object> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let object = value.into_object()?;
        let object = object.into_mut()?;
        Ok(object)
    }
}

impl FromValue for Ref<Object> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let object = value.into_object()?;
        let object = object.into_ref()?;
        Ok(object)
    }
}

impl UnsafeFromValue for &Object {
    type Output = *const Object;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let object = value.into_object()?;
        let object = object.into_ref()?;
        Ok(Ref::into_raw(object))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Object {
    type Output = *mut Object;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let object = value.into_object()?;
        let object = object.into_mut()?;
        Ok(Mut::into_raw(object))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl Named for Object {
    const NAME: RawStr = RawStr::from_str("Object");
}

impl InstallWith for Object {}

pub struct DebugStruct<'a> {
    item: &'a Item,
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

/// Helper function two compare two hashmaps of values.
pub(crate) fn map_ptr_eq<K>(
    vm: &mut Vm,
    a: &BTreeMap<K, Value>,
    b: &BTreeMap<K, Value>,
) -> Result<bool, VmError>
where
    K: cmp::Eq + cmp::Ord,
    K: hash::Hash,
{
    if a.len() != b.len() {
        return Ok(false);
    }

    for (key, a) in a.iter() {
        let b = match b.get(key) {
            Some(b) => b,
            None => return Ok(false),
        };

        if !Value::value_ptr_eq(vm, a, b)? {
            return Ok(false);
        }
    }

    Ok(true)
}
