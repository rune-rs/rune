use crate::alloc::Vec;
use crate::runtime::{FromValue, ToValue, Value, VmErrorKind, VmResult};

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
            fn from_value(value: Value) -> VmResult<Self> {
                let vec = vm_try!(vm_try!(value.into_vec()).into_ref());

                let [$($var,)*] = vec.as_slice() else {
                    return VmResult::err(VmErrorKind::ExpectedTupleLength {
                        actual: vec.len(),
                        expected: $count,
                    });
                };

                VmResult::Ok(VecTuple(($(vm_try!(<$ty>::from_value($var.clone())),)*)))
            }
        }

        impl<$($ty,)*> ToValue for VecTuple<($($ty,)*)>
        where
            $($ty: ToValue,)*
        {
            #[allow(unused_mut)]
            fn to_value(self) -> VmResult<Value> {
                let ($($var,)*) = self.0;
                let mut vec = vm_try!(Vec::try_with_capacity($count));
                $(vm_try!(vec.try_push(vm_try!($var.to_value())));)*
                Value::vec(vec)
            }
        }
    };
}

repeat_macro!(impl_from_value_tuple_vec);
