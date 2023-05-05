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
    ($count:expr $(, $ty:ident $value:ident $_:expr)*) => {
        impl<$($ty,)*> FromValue for VecTuple<($($ty,)*)>
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> VmResult<Self> {
                let vec = vm_try!(value.into_vec());
                let vec = vm_try!(vec.take());

                if vec.len() != $count {
                    return VmResult::err(VmErrorKind::ExpectedTupleLength {
                        actual: vec.len(),
                        expected: $count,
                    });
                }

                #[allow(unused_mut, unused_variables)]
                let mut it = vec.into_iter();

                $(
                    let $value: $ty = match it.next() {
                        Some(value) => vm_try!(<$ty>::from_value(value)),
                        None => {
                            return VmResult::err(VmErrorKind::IterationError);
                        },
                    };
                )*

                VmResult::Ok(VecTuple(($($value,)*)))
            }
        }

        impl<$($ty,)*> ToValue for VecTuple<($($ty,)*)>
        where
            $($ty: ToValue,)*
        {
            fn to_value(self) -> VmResult<Value> {
                let ($($value,)*) = self.0;
                let vec = vec![$(vm_try!($value.to_value()),)*];
                VmResult::Ok(Value::vec(vec))
            }
        }
    };
}

repeat_macro!(impl_from_value_tuple_vec);
