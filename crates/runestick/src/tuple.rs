use crate::{FromValue, Mut, Ref, Value, VmError};
use std::fmt;
use std::ops;

/// Struct representing a dynamic anonymous object.
#[derive(Clone)]
#[repr(transparent)]
pub struct Tuple {
    inner: Box<[Value]>,
}

impl Tuple {
    /// Convert into inner.
    pub fn into_inner(self) -> Box<[Value]> {
        self.inner
    }

    /// Get the typle at the given index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.inner.get(index)
    }
}

impl fmt::Debug for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;

        let mut it = self.inner.iter();
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

impl ops::Deref for Tuple {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl ops::DerefMut for Tuple {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl From<Vec<Value>> for Tuple {
    fn from(vec: Vec<Value>) -> Self {
        Self {
            inner: vec.into_boxed_slice(),
        }
    }
}

impl From<Box<[Value]>> for Tuple {
    fn from(inner: Box<[Value]>) -> Self {
        Self { inner }
    }
}

impl FromValue for Mut<Tuple> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_tuple()?.into_mut()?)
    }
}

impl FromValue for Ref<Tuple> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_tuple()?.into_ref()?)
    }
}

macro_rules! impl_tuple {
    () => ();

    ({$ty:ident, $var:ident, $count:expr}, $({$l_ty:ident, $l_var:ident, $l_count:expr},)*) => {
        impl_tuple!{@impl $count, {$ty, $var, $count}, $({$l_ty, $l_var, $l_count},)*}
        impl_tuple!{$({$l_ty, $l_var, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $var:ident, $ignore_count:expr},)*) => {
        impl_static_type!(impl <$($ty),*> ($($ty,)*) => $crate::TUPLE_TYPE);

        impl <$($ty,)*> $crate::FromValue for ($($ty,)*)
        where
            $($ty: $crate::FromValue,)*
        {
            fn from_value(value: $crate::Value) -> Result<Self, $crate::VmError> {
                let tuple = value.into_tuple()?.take()?;

                if tuple.len() != $count {
                    return Err($crate::VmError::from($crate::VmErrorKind::ExpectedTupleLength {
                        actual: tuple.len(),
                        expected: $count,
                    }));
                }

                #[allow(unused_mut, unused_variables)]
                let mut it = Vec::from(tuple.into_inner()).into_iter();

                $(
                    let $var = match it.next() {
                        Some(value) => <$ty>::from_value(value)?,
                        None => {
                            return Err($crate::VmError::from($crate::VmErrorKind::IterationError));
                        },
                    };
                )*

                Ok(($($var,)*))
            }
        }

        impl <$($ty,)*> $crate::ToValue for ($($ty,)*)
        where
            $($ty: $crate::ToValue,)*
        {
            fn to_value(self) -> Result<$crate::Value, $crate::VmError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_value()?;)*
                Ok($crate::Value::from($crate::Tuple::from(vec![$($var,)*])))
            }
        }
    };
}

repeat_macro!(impl_tuple);
