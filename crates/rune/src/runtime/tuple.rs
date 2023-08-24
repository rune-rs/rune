use core::fmt;
use core::ops;
use core::slice;

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{
    ConstValue, FromValue, Mut, RawMut, RawRef, Ref, Shared, ToValue, UnsafeToMut, UnsafeToRef,
    Value, VmErrorKind, VmResult,
};
use crate::Any;

/// The type of a tuple slice.
#[derive(Any)]
#[rune(builtin, static_type = TUPLE_TYPE)]
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
#[derive(Clone)]
#[repr(transparent)]
pub struct OwnedTuple {
    inner: Option<Box<[Value]>>,
}

impl OwnedTuple {
    /// Construct a new empty tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::OwnedTuple;
    ///
    /// const EMPTY: OwnedTuple = OwnedTuple::new();
    /// ```
    pub const fn new() -> Self {
        Self { inner: None }
    }

    /// Convert into inner std boxed slice.
    pub fn into_inner(self) -> Box<[Value]> {
        match self.inner {
            Some(values) => values,
            None => Box::from([]),
        }
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
        match &self.inner {
            Some(values) => Tuple::new(values),
            None => Tuple::new(&[]),
        }
    }
}

impl ops::DerefMut for OwnedTuple {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            Some(values) => Tuple::new_mut(values),
            None => Tuple::new_mut(&mut []),
        }
    }
}

impl From<Vec<Value>> for OwnedTuple {
    #[inline]
    fn from(vec: Vec<Value>) -> Self {
        Self {
            inner: if vec.is_empty() {
                None
            } else {
                Some(vec.into_boxed_slice())
            },
        }
    }
}

impl<const N: usize> From<[Value; N]> for OwnedTuple {
    #[inline]
    fn from(values: [Value; N]) -> Self {
        Self {
            inner: if values.is_empty() {
                None
            } else {
                Some(values.into())
            },
        }
    }
}

impl From<Box<[Value]>> for OwnedTuple {
    #[inline]
    fn from(inner: Box<[Value]>) -> Self {
        Self {
            inner: if inner.is_empty() { None } else { Some(inner) },
        }
    }
}

impl From<Box<[ConstValue]>> for OwnedTuple {
    fn from(inner: Box<[ConstValue]>) -> Self {
        if inner.is_empty() {
            return OwnedTuple::new();
        }

        let mut out = Vec::with_capacity(inner.len());

        for value in inner.into_vec() {
            out.push(value.into_value());
        }

        Self {
            inner: Some(out.into_boxed_slice()),
        }
    }
}

impl FromValue for OwnedTuple {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::EmptyTuple => VmResult::Ok(Self::new()),
            Value::Tuple(tuple) => VmResult::Ok(vm_try!(tuple.take())),
            actual => VmResult::err(VmErrorKind::expected::<Self>(vm_try!(actual.type_info()))),
        }
    }
}

macro_rules! impl_tuple {
    // Skip conflicting implementation with `()`.
    (0) => {
        impl_static_type!(() => crate::runtime::static_type::TUPLE_TYPE);

        impl FromValue for () {
            fn from_value(value: Value) -> VmResult<Self> {
                VmResult::Ok(vm_try!(value.into_unit()))
            }
        }

        impl ToValue for () {
            fn to_value(self) -> VmResult<Value> {
                VmResult::Ok(Value::from(()))
            }
        }
    };

    ($count:expr $(, $ty:ident $var:ident $ignore_count:expr)*) => {
        impl_static_type!(impl <$($ty),*> ($($ty,)*) => crate::runtime::static_type::TUPLE_TYPE);

        impl <$($ty,)*> FromValue for ($($ty,)*)
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> VmResult<Self> {
                let tuple = vm_try!(vm_try!(value.into_tuple()).into_ref());

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
                VmResult::Ok(Value::from(OwnedTuple::from(vec![$($var,)*])))
            }
        }
    };
}

repeat_macro!(impl_tuple);

impl FromValue for Mut<Tuple> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::EmptyTuple => {
                let tuple = vm_try!(Shared::new(OwnedTuple::new()).into_mut());
                let tuple = Mut::map(tuple, |this| &mut **this);
                VmResult::Ok(tuple)
            }
            Value::Tuple(tuple) => {
                let tuple = vm_try!(tuple.into_mut());
                let tuple = Mut::map(tuple, |this| &mut **this);
                VmResult::Ok(tuple)
            }
            actual => VmResult::err(VmErrorKind::expected::<Self>(vm_try!(actual.type_info()))),
        }
    }
}

impl FromValue for Ref<Tuple> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::EmptyTuple => VmResult::Ok(Ref::from_static(Tuple::new(&[]))),
            Value::Tuple(tuple) => {
                let tuple = vm_try!(tuple.into_ref());
                let tuple = Ref::map(tuple, |this| &**this);
                VmResult::Ok(tuple)
            }
            actual => VmResult::err(VmErrorKind::expected::<Self>(vm_try!(actual.type_info()))),
        }
    }
}

impl UnsafeToRef for Tuple {
    type Guard = Option<RawRef>;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        match value {
            Value::EmptyTuple => VmResult::Ok((Tuple::new(&[]), None)),
            Value::Tuple(tuple) => {
                let tuple = Ref::map(vm_try!(tuple.into_ref()), |tuple| &**tuple);
                let (value, guard) = Ref::into_raw(tuple);
                VmResult::Ok((value.as_ref(), Some(guard)))
            }
            actual => VmResult::err(VmErrorKind::expected::<Self>(vm_try!(actual.type_info()))),
        }
    }
}

impl UnsafeToMut for Tuple {
    type Guard = Option<RawMut>;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        match value {
            Value::EmptyTuple => VmResult::Ok((Tuple::new_mut(&mut []), None)),
            Value::Tuple(tuple) => {
                let tuple = Mut::map(vm_try!(tuple.into_mut()), |tuple| &mut **tuple);
                let (mut value, guard) = Mut::into_raw(tuple);
                VmResult::Ok((value.as_mut(), Some(guard)))
            }
            actual => VmResult::err(VmErrorKind::expected::<Self>(vm_try!(actual.type_info()))),
        }
    }
}
