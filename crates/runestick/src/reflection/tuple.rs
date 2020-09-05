//! Trait implementation for decoding tuples.

use crate::{Shared, Type, TypeInfo, Value};

macro_rules! impl_from_value_tuple {
    () => {
    };

    ({$ty:ident, $var:ident, $count:expr}, $({$l_ty:ident, $l_var:ident, $l_count:expr},)*) => {
        impl_from_value_tuple!{@impl $count, {$ty, $var, $count}, $({$l_ty, $l_var, $l_count},)*}
        impl_from_value_tuple!{$({$l_ty, $l_var, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $var:ident, $ignore_count:expr},)*) => {
        impl <$($ty,)*> $crate::ReflectValueType for ($($ty,)*) {
            type Owned = ($($ty,)*);

            fn value_type() -> Type {
                Type::StaticType($crate::TUPLE_TYPE)
            }

            fn type_info() -> TypeInfo {
                TypeInfo::StaticType($crate::TUPLE_TYPE)
            }
        }

        impl <$($ty,)*> $crate::FromValue for ($($ty,)*)
        where
            $($ty: $crate::FromValue,)*
        {
            fn from_value(value: Value) -> Result<Self, $crate::VmError> {
                let tuple = value.into_tuple()?;
                let tuple = tuple.take()?;

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
                Ok(Value::Tuple(Shared::new($crate::Tuple::from(vec![$($var,)*]))))
            }
        }
    };
}

impl_from_value_tuple!(
    {H, h, 8},
    {G, g, 7},
    {F, f, 6},
    {E, e, 5},
    {D, d, 4},
    {C, c, 3},
    {B, b, 2},
    {A, a, 1},
);
