use crate::collections::HashMap;
use crate::{
    FromValue, Mut, Named, RawMut, RawRef, RawStr, Ref, ToValue, UnsafeFromValue, Value, VmError,
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
pub type IntoIter = crate::collections::hash_map::IntoIter<String, Value>;

/// A mutable iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`iter_mut`] method on [`Object`]. See its
/// documentation for more.
///
/// [`iter_mut`]: struct.Object.html#method.iter_mut
/// [`Object`]: struct.Object.html
pub type IterMut<'a> = crate::collections::hash_map::IterMut<'a, String, Value>;

/// An iterator over the entries of a `Object`.
///
/// This `struct` is created by the [`iter`] method on [`Object`]. See its
/// documentation for more.
///
/// [`iter`]: struct.Object.html#method.iter
/// [`Object`]: struct.Object.html
pub type Iter<'a> = crate::collections::hash_map::Iter<'a, String, Value>;

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
    inner: HashMap<String, Value>,
}

impl Object {
    /// Construct a new object.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Returns the number of elements in the object.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the Object contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Value>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq,
    {
        self.inner.get(k)
    }

    /// Get the given value at the given index.
    pub fn get_value<Q: ?Sized, T>(&self, k: &Q) -> Result<Option<T>, VmError>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq,
        T: FromValue,
    {
        let value = match self.inner.get(k) {
            Some(value) => value.clone(),
            None => return Ok(None),
        };

        Ok(Some(T::from_value(value)?))
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq,
    {
        self.inner.get_mut(k)
    }

    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq,
    {
        self.inner.contains_key(k)
    }

    /// Removes a key from the object, returning the value at the key if the key
    /// was previously in the object.
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<Value>
    where
        String: borrow::Borrow<Q>,
        Q: hash::Hash + cmp::Eq,
    {
        self.inner.remove(k)
    }

    /// Inserts a key-value pair into the dynamic object, converting it as
    /// necessary through the [`ToValue`] trait.
    pub fn insert_value<T>(&mut self, k: String, v: T) -> Result<(), VmError>
    where
        T: ToValue,
    {
        self.inner.insert(k, v.to_value()?);
        Ok(())
    }

    /// Inserts a key-value pair into the dynamic object.
    pub fn insert(&mut self, k: String, v: Value) -> Option<Value> {
        self.inner.insert(k, v)
    }

    /// Clears the object, removing all key-value pairs. Keeps the allocated
    /// memory for reuse.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Construct a new object with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(cap),
        }
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

    /// An iterator visiting all key-value pairs in arbitrary order,
    /// with mutable references to the values.
    /// The iterator element type is `(&'a String, &'a mut Value)`.
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        self.inner.iter_mut()
    }

    /// Value pointer equals implementation for an Object.
    pub(crate) fn value_ptr_eq(a: &Self, b: &Self) -> Result<bool, VmError> {
        if a.len() != b.len() {
            return Ok(false);
        }

        for (key, a) in a.iter() {
            let b = match b.get(key) {
                Some(b) => b,
                None => return Ok(false),
            };

            if !Value::value_ptr_eq(a, b)? {
                return Ok(false);
            }
        }

        Ok(true)
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

impl From<HashMap<String, Value>> for Object {
    fn from(object: HashMap<String, Value>) -> Self {
        Self { inner: object }
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
