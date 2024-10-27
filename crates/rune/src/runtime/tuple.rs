use core::fmt;
use core::ops;
use core::slice;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::alloc::{self, Box};
use crate::runtime::{
    ConstValue, FromValue, Mut, Mutable, OwnedValue, RawAnyGuard, Ref, ToValue, UnsafeToMut,
    UnsafeToRef, Value, ValueShared, VmErrorKind, VmResult,
};
#[cfg(feature = "alloc")]
use crate::runtime::{Hasher, ProtocolCaller};
use crate::Any;

use super::Inline;

/// The type of a tuple slice.
#[derive(Any)]
#[rune(builtin, static_type = TUPLE)]
#[repr(transparent)]
pub struct Tuple {
    values: [Value],
}

impl Tuple {
    /// Construct a new tuple slice from a reference.
    pub const fn new(values: &[Value]) -> &Self {
        // SAFETY: Tuple is repr transparent over [Value].
        unsafe { &*(values as *const _ as *const Self) }
    }

    /// Construct a new tuple slice from a mutable reference.
    pub fn new_mut(values: &mut [Value]) -> &mut Self {
        // SAFETY: Tuple is repr transparent over [Value].
        unsafe { &mut *(values as *mut _ as *mut Self) }
    }

    /// Get the given value at the given index.
    pub fn get_value<T>(&self, index: usize) -> VmResult<Option<T>>
    where
        T: FromValue,
    {
        let value = match self.values.get(index) {
            Some(value) => value.clone(),
            None => return VmResult::Ok(None),
        };

        VmResult::Ok(Some(vm_try!(T::from_value(value))))
    }

    pub(crate) fn hash_with(
        &self,
        hasher: &mut Hasher,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        for value in self.values.iter() {
            vm_try!(value.hash_with(hasher, caller));
        }

        VmResult::Ok(())
    }
}

impl ops::Deref for Tuple {
    type Target = [Value];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl ops::DerefMut for Tuple {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<'a> IntoIterator for &'a Tuple {
    type Item = &'a Value;
    type IntoIter = slice::Iter<'a, Value>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Tuple {
    type Item = &'a mut Value;
    type IntoIter = slice::IterMut<'a, Value>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Struct representing a dynamic anonymous object.
///
/// To access borrowed values of a tuple in native functions, use [`Tuple`].
#[repr(transparent)]
pub struct OwnedTuple {
    inner: Box<[Value]>,
}

impl OwnedTuple {
    /// Construct a new empty tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::OwnedTuple;
    ///
    /// let empty = OwnedTuple::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Box::default(),
        }
    }

    /// Convert into inner std boxed slice.
    pub fn into_inner(self) -> Box<[Value]> {
        self.inner
    }
}

impl Default for OwnedTuple {
    fn default() -> Self {
        Self::new()
    }
}

impl TryClone for OwnedTuple {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
        })
    }

    #[inline]
    fn try_clone_from(&mut self, source: &Self) -> alloc::Result<()> {
        self.inner.try_clone_from(&source.inner)
    }
}

impl fmt::Debug for OwnedTuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;

        let mut it = self.iter();
        let last = it.next_back();

        for el in it {
            write!(f, "{:?}, ", el)?;
        }

        if let Some(last) = last {
            write!(f, "{:?}", last)?;
        }

        write!(f, ")")?;
        Ok(())
    }
}

impl ops::Deref for OwnedTuple {
    type Target = Tuple;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Tuple::new(&self.inner)
    }
}

impl ops::DerefMut for OwnedTuple {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Tuple::new_mut(&mut self.inner)
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::vec::Vec<Value>> for OwnedTuple {
    type Error = alloc::Error;

    #[inline]
    fn try_from(vec: ::rust_alloc::vec::Vec<Value>) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: alloc::Box::try_from(vec.into_boxed_slice())?,
        })
    }
}

impl TryFrom<alloc::Vec<Value>> for OwnedTuple {
    type Error = alloc::Error;

    #[inline]
    fn try_from(vec: alloc::Vec<Value>) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: vec.try_into_boxed_slice()?,
        })
    }
}

impl<const N: usize> TryFrom<[Value; N]> for OwnedTuple {
    type Error = alloc::Error;

    #[inline]
    fn try_from(values: [Value; N]) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: values.try_into()?,
        })
    }
}

impl From<alloc::Box<[Value]>> for OwnedTuple {
    #[inline]
    fn from(inner: alloc::Box<[Value]>) -> Self {
        Self { inner }
    }
}

impl TryFrom<alloc::Box<[ConstValue]>> for OwnedTuple {
    type Error = alloc::Error;

    fn try_from(inner: alloc::Box<[ConstValue]>) -> alloc::Result<Self> {
        if inner.is_empty() {
            return Ok(OwnedTuple::new());
        }

        let mut out = alloc::Vec::try_with_capacity(inner.len())?;

        for value in inner.iter() {
            out.try_push(value.to_value()?)?;
        }

        Ok(Self {
            inner: out.try_into_boxed_slice()?,
        })
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::boxed::Box<[Value]>> for OwnedTuple {
    type Error = alloc::Error;

    #[inline]
    fn try_from(inner: ::rust_alloc::boxed::Box<[Value]>) -> alloc::Result<Self> {
        Ok(Self {
            inner: alloc::Box::try_from(inner)?,
        })
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::boxed::Box<[ConstValue]>> for OwnedTuple {
    type Error = alloc::Error;

    fn try_from(inner: ::rust_alloc::boxed::Box<[ConstValue]>) -> alloc::Result<Self> {
        if inner.is_empty() {
            return Ok(OwnedTuple::new());
        }

        let mut out = alloc::Vec::try_with_capacity(inner.len())?;

        for value in inner.iter() {
            out.try_push(value.to_value()?)?;
        }

        Ok(Self {
            inner: out.try_into_boxed_slice()?,
        })
    }
}

impl FromValue for OwnedTuple {
    fn from_value(value: Value) -> VmResult<Self> {
        match vm_try!(value.take_value()) {
            OwnedValue::Inline(value) => match value {
                Inline::Unit => VmResult::Ok(Self::new()),
                actual => VmResult::expected::<Self>(actual.type_info()),
            },
            OwnedValue::Mutable(value) => match value {
                Mutable::Tuple(tuple) => VmResult::Ok(tuple),
                actual => VmResult::expected::<Self>(actual.type_info()),
            },
            OwnedValue::Any(value) => VmResult::expected::<Self>(value.type_info()),
        }
    }
}

macro_rules! impl_tuple {
    // Skip conflicting implementation with `()`.
    (0) => {
        impl_static_type!((), crate::runtime::static_type::TUPLE, crate::runtime::static_type::TUPLE_HASH);

        impl FromValue for () {
            fn from_value(value: Value) -> VmResult<Self> {
                VmResult::Ok(vm_try!(value.into_unit()))
            }
        }

        impl ToValue for () {
            fn to_value(self) -> VmResult<Value> {
                VmResult::Ok(Value::unit())
            }
        }
    };

    ($count:expr $(, $ty:ident $var:ident $ignore_count:expr)*) => {
        impl_static_type!(impl <$($ty),*> ($($ty,)*), crate::runtime::static_type::TUPLE, crate::runtime::static_type::TUPLE_HASH);

        impl <$($ty,)*> FromValue for ($($ty,)*)
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> VmResult<Self> {
                let tuple = vm_try!(value.into_tuple_ref());

                let [$($var,)*] = &tuple[..] else {
                    return VmResult::err(VmErrorKind::ExpectedTupleLength {
                        actual: tuple.len(),
                        expected: $count,
                    });
                };

                VmResult::Ok(($(vm_try!(<$ty>::from_value($var.clone())),)*))
            }
        }

        impl <$($ty,)*> ToValue for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            fn to_value(self) -> VmResult<Value> {
                let ($($var,)*) = self;
                $(let $var = vm_try!($var.to_value());)*
                let mut vec = vm_try!(alloc::Vec::try_with_capacity($count));
                $(vm_try!(vec.try_push($var));)*
                VmResult::Ok(vm_try!(Value::try_from(vm_try!(OwnedTuple::try_from(vec)))))
            }
        }
    };
}

repeat_macro!(impl_tuple);

impl FromValue for Ref<Tuple> {
    fn from_value(value: Value) -> VmResult<Self> {
        let result = match vm_try!(value.into_value_shared()) {
            ValueShared::Inline(value) => match value {
                Inline::Unit => Ok(Ref::from_static(Tuple::new(&[]))),
                actual => Err(actual.type_info()),
            },
            ValueShared::Mutable(value) => {
                let value = vm_try!(value.into_ref());

                let result = Ref::try_map(value, |value| match value {
                    Mutable::Tuple(tuple) => Some(&**tuple),
                    _ => None,
                });

                match result {
                    Ok(tuple) => Ok(tuple),
                    Err(actual) => Err(actual.type_info()),
                }
            }
            ValueShared::Any(value) => Err(value.type_info()),
        };

        match result {
            Ok(tuple) => VmResult::Ok(tuple),
            Err(actual) => VmResult::expected::<Self>(actual),
        }
    }
}

impl FromValue for Mut<Tuple> {
    fn from_value(value: Value) -> VmResult<Self> {
        let result = match vm_try!(value.into_value_shared()) {
            ValueShared::Inline(value) => match value {
                Inline::Unit => Ok(Mut::from_static(Tuple::new_mut(&mut []))),
                actual => Err(actual.type_info()),
            },
            ValueShared::Mutable(value) => {
                let value = vm_try!(value.clone().into_mut());

                let result = Mut::try_map(value, |kind| match kind {
                    Mutable::Tuple(tuple) => Some(&mut **tuple),
                    _ => None,
                });

                match result {
                    Ok(value) => Ok(value),
                    Err(actual) => Err(actual.type_info()),
                }
            }
            ValueShared::Any(value) => Err(value.type_info()),
        };

        match result {
            Ok(tuple) => VmResult::Ok(tuple),
            Err(actual) => VmResult::expected::<Self>(actual),
        }
    }
}

impl UnsafeToRef for Tuple {
    type Guard = RawAnyGuard;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let (value, guard) = Ref::into_raw(vm_try!(Ref::from_value(value)));
        VmResult::Ok((value.as_ref(), guard))
    }
}

impl UnsafeToMut for Tuple {
    type Guard = RawAnyGuard;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        let (mut value, guard) = Mut::into_raw(vm_try!(Mut::from_value(value)));
        VmResult::Ok((value.as_mut(), guard))
    }
}
