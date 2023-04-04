use crate::compile::{InstallWith, Named};
use crate::runtime::{
    FromValue, Iterator, Mut, RawMut, RawRef, RawStr, Ref, Shared, ToValue, UnsafeFromValue, Value,
    Vm, VmError, VmErrorKind, VmResult,
};
use std::cmp;
use std::fmt;
use std::ops;
use std::slice;
use std::vec;

/// Struct representing a dynamic vector.
///
/// # Examples
///
/// ```
/// # fn main() -> rune::Result<()> {
/// let mut vec = rune::runtime::Vec::new();
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

    /// Sort the vector with the given comparison function.
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Value, &Value) -> cmp::Ordering,
    {
        self.inner.sort_by(compare)
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

    /// Set by index
    pub fn set(&mut self, index: usize, value: Value) -> VmResult<()> {
        if index >= self.len() {
            VmResult::Err(VmError::from(VmErrorKind::OutOfRange {
                index: index.into(),
                len: self.len().into(),
            }))
        } else {
            self.inner[index] = value;
            VmResult::Ok(())
        }
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
        self.inner.push(value.to_value().into_result()?);
        Ok(())
    }

    /// Get the value at the given index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.inner.get(index)
    }

    /// Get the given value at the given index.
    pub fn get_value<T>(&self, index: usize) -> VmResult<Option<T>>
    where
        T: FromValue,
    {
        let value = match self.inner.get(index) {
            Some(value) => value.clone(),
            None => return VmResult::Ok(None),
        };

        VmResult::Ok(Some(vm_try!(T::from_value(value))))
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

    /// Removes the element at the specified index from a dynamic vector.
    pub fn remove(&mut self, index: usize) {
        self.inner.remove(index);
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// vector.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Inserts an element at position index within the vector, shifting all
    /// elements after it to the right.
    pub fn insert(&mut self, index: usize, value: Value) {
        self.inner.insert(index, value);
    }

    /// Extend this vector with something that implements the into_iter
    /// protocol.
    pub fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            self.push(value);
        }

        VmResult::Ok(())
    }

    /// Convert into a rune iterator.
    pub fn into_iterator(&self) -> Iterator {
        Iterator::from_double_ended("std::vec::Iter", self.clone().into_iter())
    }

    /// Compare two vectors for equality.
    pub(crate) fn value_ptr_eq(vm: &mut Vm, a: &Self, b: &Self) -> VmResult<bool> {
        if a.len() != b.len() {
            return VmResult::Ok(false);
        }

        for (a, b) in a.iter().zip(b.iter()) {
            if !vm_try!(Value::value_ptr_eq(vm, a, b)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }
}

impl Named for Vec {
    const BASE_NAME: RawStr = RawStr::from_str("Vec");
}

impl InstallWith for Vec {}

impl fmt::Debug for Vec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&*self.inner).finish()
    }
}

impl ops::Deref for Vec {
    type Target = [Value];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for Vec {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
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
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(vm_try!(value.into_vec()).into_mut()))
    }
}

impl FromValue for Ref<Vec> {
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(vm_try!(value.into_vec()).into_ref()))
    }
}

impl FromValue for Vec {
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(vm_try!(value.into_vec()).take()))
    }
}

impl<T> FromValue for vec::Vec<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> VmResult<Self> {
        let vec = vm_try!(value.into_vec());
        let vec = vm_try!(vec.take());

        let mut output = vec::Vec::with_capacity(vec.len());

        for value in vec {
            output.push(vm_try!(T::from_value(value)));
        }

        VmResult::Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a [Value] {
    type Output = *const [Value];
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let vec = vm_try!(value.into_vec());
        let (vec, guard) = Ref::into_raw(vm_try!(vec.into_ref()));
        // Safety: we're holding onto the guard for the vector here, so it is
        // live.
        VmResult::Ok((unsafe { &**vec }, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a Vec {
    type Output = *const Vec;
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let vec = vm_try!(value.into_vec());
        VmResult::Ok(Ref::into_raw(vm_try!(vec.into_ref())))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Vec {
    type Output = *mut Vec;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let vec = vm_try!(value.into_vec());
        VmResult::Ok(Mut::into_raw(vm_try!(vec.into_mut())))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<T> ToValue for vec::Vec<T>
where
    T: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let mut vec = vec::Vec::with_capacity(self.len());

        for value in self {
            vec.push(vm_try!(value.to_value()));
        }

        VmResult::Ok(Value::from(Shared::new(Vec::from(vec))))
    }
}
