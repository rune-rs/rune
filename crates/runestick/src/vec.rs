use crate::{
    FromValue, Mut, Named, RawMut, RawRef, RawStr, Ref, Shared, ToValue, UnsafeFromValue, Value,
    VmError,
};
use std::fmt;
use std::ops;
use std::slice;
use std::vec;

/// Struct representing a dynamic vector.
///
/// # Examples
///
/// ```rust
/// # fn main() -> runestick::Result<()> {
/// let mut vec = runestick::Vec::new();
/// assert!(vec.is_empty());
///
/// vec.push_value(42)?;
/// vec.push_value(true)?;
/// assert_eq!(2, vec.len());
///
/// assert_eq!(Some(42), vec.get_value(0)?);
/// assert_eq!(Some(true), vec.get_value(1)?);
/// assert_eq!(None::<bool>, vec.get_value(2)?);
/// # Ok(()) }
/// ```
#[derive(Clone)]
#[repr(transparent)]
pub struct Vec {
    inner: vec::Vec<Value>,
}

impl Vec {
    /// Construct a new empty dynamic vector.
    pub const fn new() -> Self {
        Self {
            inner: vec::Vec::new(),
        }
    }

    /// Construct a new dynamic vector guaranteed to have at least the given
    /// capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: vec::Vec::with_capacity(cap),
        }
    }

    /// Convert into inner std vector.
    pub fn into_inner(self) -> vec::Vec<Value> {
        self.inner
    }

    /// Returns `true` if the dynamic vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of elements in the dynamic vector, also referred to
    /// as its 'length'.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Appends an element to the back of a dynamic vector.
    pub fn push(&mut self, value: Value) {
        self.inner.push(value);
    }

    /// Appends an element to the back of a dynamic vector, converting it as
    /// necessary through the [`ToValue`] trait.
    pub fn push_value<T>(&mut self, value: T) -> Result<(), VmError>
    where
        T: ToValue,
    {
        self.inner.push(value.to_value()?);
        Ok(())
    }

    /// Get the value at the given index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.inner.get(index)
    }

    /// Get the given value at the given index.
    pub fn get_value<T>(&self, index: usize) -> Result<Option<T>, VmError>
    where
        T: FromValue,
    {
        let value = match self.inner.get(index) {
            Some(value) => value.clone(),
            None => return Ok(None),
        };

        Ok(Some(T::from_value(value)?))
    }

    /// Get the mutable value at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.inner.get_mut(index)
    }

    /// Removes the last element from a dynamic vector and returns it, or
    /// [`None`] if it is empty.
    pub fn pop(&mut self) -> Option<Value> {
        self.inner.pop()
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// vector.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Convert into a runestick iterator.
    pub fn into_iterator(&self) -> crate::Iterator {
        crate::Iterator::from_double_ended("std::vec::Iter", self.clone().into_iter())
    }
}

impl Named for Vec {
    const NAME: RawStr = RawStr::from_str("Vec");
}

impl fmt::Debug for Vec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&*self.inner).finish()
    }
}

impl ops::Deref for Vec {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl ops::DerefMut for Vec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl IntoIterator for Vec {
    type Item = Value;
    type IntoIter = vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a Vec {
    type Item = &'a Value;
    type IntoIter = slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for &'a mut Vec {
    type Item = &'a mut Value;
    type IntoIter = slice::IterMut<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

impl From<vec::Vec<Value>> for Vec {
    fn from(inner: vec::Vec<Value>) -> Self {
        Self { inner }
    }
}

impl From<Box<[Value]>> for Vec {
    fn from(inner: Box<[Value]>) -> Self {
        Self {
            inner: inner.to_vec(),
        }
    }
}

impl FromValue for Mut<Vec> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_vec()?.into_mut()?)
    }
}

impl FromValue for Ref<Vec> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_vec()?.into_ref()?)
    }
}

impl FromValue for Vec {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_vec()?.take()?)
    }
}

impl<T> FromValue for vec::Vec<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        let vec = value.into_vec()?;
        let vec = vec.take()?;

        let mut output = vec::Vec::with_capacity(vec.len());

        for value in vec {
            output.push(T::from_value(value)?);
        }

        Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a [Value] {
    type Output = *const [Value];
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        let (vec, guard) = Ref::into_raw(vec.into_ref()?);
        // Safety: we're holding onto the guard for the vector here, so it is
        // live.
        Ok((unsafe { &**vec }, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a Vec {
    type Output = *const Vec;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(Ref::into_raw(vec.into_ref()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Vec {
    type Output = *mut Vec;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(Mut::into_raw(vec.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<T> ToValue for vec::Vec<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let mut vec = vec::Vec::with_capacity(self.len());

        for value in self {
            vec.push(value.to_value()?);
        }

        Ok(Value::from(Shared::new(Vec::from(vec))))
    }
}
