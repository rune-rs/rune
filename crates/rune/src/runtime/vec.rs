mod iter;

use core::cmp;
use core::cmp::Ordering;
use core::fmt;
use core::ops;
use core::slice;
use core::slice::SliceIndex;

use crate::no_std::prelude::*;
use crate::no_std::vec;

use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{
    FromValue, Iterator, ProtocolCaller, RawRef, RawStr, Ref, Shared, ToValue, UnsafeToRef, Value,
    VmErrorKind, VmResult,
};

use self::iter::Iter;

/// Struct representing a dynamic vector.
///
/// # Examples
///
/// ```
/// let mut vec = rune::runtime::Vec::new();
/// assert!(vec.is_empty());
///
/// vec.push_value(42).into_result()?;
/// vec.push_value(true).into_result()?;
/// assert_eq!(2, vec.len());
///
/// assert_eq!(Some(42), vec.get_value(0).into_result()?);
/// assert_eq!(Some(true), vec.get_value(1).into_result()?);
/// assert_eq!(None::<bool>, vec.get_value(2).into_result()?);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Clone)]
#[repr(transparent)]
pub struct Vec {
    inner: vec::Vec<Value>,
}

impl Vec {
    /// Constructs a new, empty dynamic `Vec`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Vec;
    ///
    /// let mut vec = Vec::new();
    /// ```
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

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Value, Vec};
    ///
    /// let mut v = Vec::new();
    /// assert!(v.is_empty());
    ///
    /// v.push(Value::Integer(1));
    /// assert!(!v.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of elements in the dynamic vector, also referred to
    /// as its 'length'.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the number of elements in the dynamic vector, also referred to
    /// as its 'length'.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Set by index
    pub fn set(&mut self, index: usize, value: Value) -> VmResult<()> {
        let Some(v) = self.inner.get_mut(index) else {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.len().into(),
            });
        };

        *v = value;
        VmResult::Ok(())
    }

    /// Appends an element to the back of a dynamic vector.
    pub fn push(&mut self, value: Value) {
        self.inner.push(value);
    }

    /// Appends an element to the back of a dynamic vector, converting it as
    /// necessary through the [`ToValue`] trait.
    pub fn push_value<T>(&mut self, value: T) -> VmResult<()>
    where
        T: ToValue,
    {
        self.inner.push(vm_try!(value.to_value()));
        VmResult::Ok(())
    }

    /// Get the value at the given index.
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[Value]>,
    {
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
    pub fn remove(&mut self, index: usize) -> Value {
        self.inner.remove(index)
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
    pub fn iter_ref(this: Ref<Self>) -> Iterator {
        Iterator::from_double_ended("std::vec::Iter", Iter::new(this))
    }

    /// Access the inner values as a slice.
    pub(crate) fn as_slice(&self) -> &[Value] {
        &self.inner
    }

    pub(crate) fn partial_eq_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        if a.len() != b.len() {
            return VmResult::Ok(false);
        }

        for (a, b) in a.iter().zip(b.iter()) {
            if !vm_try!(Value::partial_eq_with(a, b, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if a.len() != b.len() {
            return VmResult::Ok(false);
        }

        for (a, b) in a.iter().zip(b.iter()) {
            if !vm_try!(Value::eq_with(a, b, caller)) {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    pub(crate) fn partial_cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        let mut b = b.inner.iter();

        for a in a.inner.iter() {
            let Some(b) = b.next() else {
                return VmResult::Ok(Some(Ordering::Greater));
            };

            match vm_try!(Value::partial_cmp_with(a, b, caller)) {
                Some(Ordering::Equal) => continue,
                other => return VmResult::Ok(other),
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
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        let mut b = b.inner.iter();

        for a in a.inner.iter() {
            let Some(b) = b.next() else {
                return VmResult::Ok(Ordering::Greater);
            };

            match vm_try!(Value::cmp_with(a, b, caller)) {
                Ordering::Equal => continue,
                other => return VmResult::Ok(other),
            }
        }

        if b.next().is_some() {
            return VmResult::Ok(Ordering::Less);
        }

        VmResult::Ok(Ordering::Equal)
    }
}

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

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a Vec {
    type Item = &'a Value;
    type IntoIter = slice::Iter<'a, Value>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for &'a mut Vec {
    type Item = &'a mut Value;
    type IntoIter = slice::IterMut<'a, Value>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

impl From<vec::Vec<Value>> for Vec {
    #[inline]
    fn from(inner: vec::Vec<Value>) -> Self {
        Self { inner }
    }
}

impl From<Box<[Value]>> for Vec {
    #[inline]
    fn from(inner: Box<[Value]>) -> Self {
        Self {
            inner: inner.to_vec(),
        }
    }
}

impl Named for Vec {
    const BASE_NAME: RawStr = RawStr::from_str("Vec");
}

impl InstallWith for Vec {}

from_value!(Vec, into_vec);

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

impl UnsafeToRef for [Value] {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let vec = vm_try!(value.into_vec());
        let (vec, guard) = Ref::into_raw(vm_try!(vec.into_ref()));
        // Safety: we're holding onto the guard for the vector here, so it is
        // live.
        VmResult::Ok(((*vec).as_slice(), guard))
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
