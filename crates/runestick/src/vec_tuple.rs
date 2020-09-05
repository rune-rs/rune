use crate::{FromValue, ToValue, Value, VmError, VmErrorKind};

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
    () => {
    };

    ({$ty:ident, $value:ident, $count:expr}, $({$rest_ty:ident, $rest_value:ident, $rest_count:expr},)*) => {
        impl_from_value_tuple_vec!{@impl $count, {$ty, $value, $count}, $({$rest_ty, $rest_value, $rest_count},)*}
        impl_from_value_tuple_vec!{$({$rest_ty, $rest_value, $rest_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $value:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> FromValue for VecTuple<($($ty,)*)>
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> Result<Self, VmError> {
                let vec = value.into_vec()?;
                let vec = vec.take()?;

                if vec.len() != $count {
                    return Err(VmError::from(VmErrorKind::ExpectedTupleLength {
                        actual: vec.len(),
                        expected: $count,
                    }));
                }

                #[allow(unused_mut, unused_variables)]
                let mut it = vec.into_iter();

                $(
                    let $value: $ty = match it.next() {
                        Some(value) => <$ty>::from_value(value)?,
                        None => {
                            return Err(VmError::from(VmErrorKind::IterationError));
                        },
                    };
                )*

                Ok(VecTuple(($($value,)*)))
            }
        }

        impl<$($ty,)*> ToValue for VecTuple<($($ty,)*)>
        where
            $($ty: ToValue,)*
        {
            fn to_value(self) -> Result<Value, VmError> {
                let ($($value,)*) = self.0;
                let vec = vec![$($value.to_value()?,)*];
                Ok(Value::vec(vec))
            }
        }
    };
}

impl_from_value_tuple_vec!(
    {H, h, 8},
    {G, g, 7},
    {F, f, 6},
    {E, e, 5},
    {D, d, 4},
    {C, c, 3},
    {B, b, 2},
    {A, a, 1},
);
