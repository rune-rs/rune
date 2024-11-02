use core::borrow::Borrow;
use core::fmt;
use core::ops::{self, Deref, DerefMut};
use core::slice;

use crate as rune;
use crate::alloc::alloc::Global;
use crate::alloc::borrow::TryToOwned;
use crate::alloc::clone::TryClone;
use crate::alloc::fmt::TryWrite;
use crate::alloc::{self, Box};
use crate::Any;

use super::{
    ConstValue, EmptyConstContext, Formatter, FromConstValue, FromValue, Mut, RawAnyGuard, Ref,
    RuntimeError, ToConstValue, ToValue, UnsafeToMut, UnsafeToRef, Value, VmErrorKind, VmResult,
};
#[cfg(feature = "alloc")]
use super::{Hasher, ProtocolCaller};

/// The type of a tuple slice.
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

    /// Construct a boxed tuple over a boxed slice of values.
    pub(crate) fn from_boxed(boxed: Box<[Value]>) -> Box<Self> {
        let (values, Global) = Box::into_raw_with_allocator(boxed);
        // SAFETY: Tuple is repr transparent over [Value].
        unsafe { Box::from_raw_in(values as *mut Tuple, Global) }
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

    pub(crate) fn debug_fmt_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        let mut it = self.iter().peekable();
        vm_try!(vm_write!(f, "("));

        while let Some(value) = it.next() {
            vm_try!(value.debug_fmt_with(f, caller));

            if it.peek().is_some() {
                vm_try!(vm_write!(f, ", "));
            }
        }

        vm_try!(vm_write!(f, ")"));
        VmResult::Ok(())
    }

    pub(crate) fn clone_with(&self, caller: &mut dyn ProtocolCaller) -> VmResult<OwnedTuple> {
        let mut vec = vm_try!(alloc::Vec::try_with_capacity(self.len()));

        for value in self.values.iter() {
            let value = vm_try!(value.clone_with(caller));
            vm_try!(vec.try_push(value));
        }

        VmResult::Ok(vm_try!(OwnedTuple::try_from(vec)))
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
#[derive(Any)]
#[rune(item = ::std::tuple, name = Tuple)]
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

    /// Coerce this owned tuple into a boxed tuple.
    pub fn into_boxed_tuple(self) -> Box<Tuple> {
        Tuple::from_boxed(self.inner)
    }

    /// Convert into inner std boxed slice.
    pub fn into_inner(self) -> Box<[Value]> {
        self.inner
    }
}

impl Deref for OwnedTuple {
    type Target = Tuple;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Tuple::new(&self.inner)
    }
}

impl DerefMut for OwnedTuple {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Tuple::new_mut(&mut self.inner)
    }
}

impl AsRef<Tuple> for OwnedTuple {
    #[inline]
    fn as_ref(&self) -> &Tuple {
        self
    }
}

impl AsMut<Tuple> for OwnedTuple {
    #[inline]
    fn as_mut(&mut self) -> &mut Tuple {
        self
    }
}

impl Borrow<Tuple> for OwnedTuple {
    #[inline]
    fn borrow(&self) -> &Tuple {
        self
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
    type Error = RuntimeError;

    fn try_from(inner: alloc::Box<[ConstValue]>) -> Result<Self, RuntimeError> {
        if inner.is_empty() {
            return Ok(OwnedTuple::new());
        }

        let mut out = alloc::Vec::try_with_capacity(inner.len())?;

        for value in inner.iter() {
            out.try_push(value.to_value_with(&EmptyConstContext)?)?;
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
    type Error = RuntimeError;

    fn try_from(inner: ::rust_alloc::boxed::Box<[ConstValue]>) -> Result<Self, RuntimeError> {
        if inner.is_empty() {
            return Ok(OwnedTuple::new());
        }

        let mut out = alloc::Vec::try_with_capacity(inner.len())?;

        for value in inner.iter() {
            out.try_push(value.to_value_with(&EmptyConstContext)?)?;
        }

        Ok(Self {
            inner: out.try_into_boxed_slice()?,
        })
    }
}

macro_rules! impl_tuple {
    // Skip conflicting implementation with `()`.
    (0) => {
        impl_one_builtin_type_of!(impl ::std::tuple::Tuple, ());

        impl FromValue for () {
            #[inline]
            fn from_value(value: Value) -> Result<Self, RuntimeError> {
                value.into_unit()
            }
        }

        impl ToValue for () {
            #[inline]
            fn to_value(self) -> Result<Value, RuntimeError> {
                Ok(Value::unit())
            }
        }
    };

    ($count:expr $(, $ty:ident $var:ident $ignore_count:expr)*) => {
        impl_one_builtin_type_of!(impl <$($ty),*> ::std::tuple::Tuple, ($($ty,)*));

        impl <$($ty,)*> FromValue for ($($ty,)*)
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> Result<Self, RuntimeError> {
                let tuple = value.into_tuple_ref()?;

                let [$($var,)*] = &tuple[..] else {
                    return Err(RuntimeError::new(VmErrorKind::ExpectedTupleLength {
                        actual: tuple.len(),
                        expected: $count,
                    }));
                };

                Ok(($(<$ty as FromValue>::from_value($var.clone())?,)*))
            }
        }

        impl <$($ty,)*> FromConstValue for ($($ty,)*)
        where
            $($ty: FromConstValue,)*
        {
            fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
                let tuple = value.into_tuple()?;

                let [$($var,)*] = match <Box<[ConstValue; $count]>>::try_from(tuple) {
                    Ok(tuple) => Box::into_inner(tuple),
                    Err(tuple) => {
                        return Err(RuntimeError::new(VmErrorKind::ExpectedTupleLength {
                            actual: tuple.len(),
                            expected: $count,
                        }));
                    }
                };

                Ok(($(<$ty as FromConstValue>::from_const_value($var)?,)*))
            }
        }

        impl <$($ty,)*> ToValue for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            fn to_value(self) -> Result<Value, RuntimeError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_value()?;)*
                let mut vec = alloc::Vec::try_with_capacity($count)?;
                $(vec.try_push($var)?;)*
                let tuple = OwnedTuple::try_from(vec)?;
                Ok(Value::try_from(tuple)?)
            }
        }

        impl <$($ty,)*> ToConstValue for ($($ty,)*)
        where
            $($ty: ToConstValue,)*
        {
            fn to_const_value(self) -> Result<ConstValue, RuntimeError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_const_value()?;)*
                let mut vec = alloc::Vec::try_with_capacity($count)?;
                $(vec.try_push($var)?;)*
                let tuple = Box::<[ConstValue]>::try_from(vec)?;
                Ok(ConstValue::tuple(tuple))
            }
        }
    };
}

repeat_macro!(impl_tuple);

impl FromValue for Box<Tuple> {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_tuple()
    }
}

impl FromValue for Ref<Tuple> {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_tuple_ref()
    }
}

impl FromValue for Mut<Tuple> {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_tuple_mut()
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

impl TryToOwned for Tuple {
    type Owned = OwnedTuple;

    #[inline]
    fn try_to_owned(&self) -> alloc::Result<Self::Owned> {
        let mut vec = alloc::Vec::try_with_capacity(self.len())?;

        for value in self.iter() {
            vec.try_push(value.clone())?;
        }

        OwnedTuple::try_from(vec)
    }
}
