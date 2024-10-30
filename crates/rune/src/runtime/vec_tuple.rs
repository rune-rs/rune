use crate::alloc::Vec;
use crate::runtime::{FromValue, RuntimeError, ToValue, Value, VmErrorKind};

/// A helper type to deserialize arrays with different interior types.
///
/// This implements [FromValue], allowing it to be used as a return value from
/// a virtual machine.
///
/// [FromValue]: crate::FromValue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VecTuple<T>(pub T);

impl<T> VecTuple<T>
where
    Self: ToValue,
{
    /// Construct a new vector tuple for serializing values.
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

macro_rules! impl_from_value_tuple_vec {
    ($count:expr $(, $ty:ident $var:ident $_:expr)*) => {
        impl<$($ty,)*> FromValue for VecTuple<($($ty,)*)>
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> Result<Self, RuntimeError> {
                let vec = value.into_any_ref::<$crate::runtime::Vec>()?;

                let [$($var,)*] = vec.as_slice() else {
                    return Err(RuntimeError::new(VmErrorKind::ExpectedTupleLength {
                        actual: vec.len(),
                        expected: $count,
                    }));
                };

                Ok(VecTuple(($(<$ty>::from_value($var.clone())?,)*)))
            }
        }

        impl<$($ty,)*> ToValue for VecTuple<($($ty,)*)>
        where
            $($ty: ToValue,)*
        {
            #[allow(unused_mut)]
            fn to_value(self) -> Result<Value, RuntimeError> {
                let ($($var,)*) = self.0;
                let mut vec = Vec::try_with_capacity($count)?;

                $(
                    let $var = $var.to_value()?;
                    vec.try_push($var)?;
                )*

                Ok(Value::vec(vec)?)
            }
        }
    };
}

repeat_macro!(impl_from_value_tuple_vec);
