//! Trait implementation for decoding tuples.

use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};

impl ReflectValueType for () {
    type Owned = ();

    fn value_type() -> ValueType {
        ValueType::Unit
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Unit
    }
}

impl ToValue for () {
    fn to_value(self, _vm: &mut Vm) -> Result<Value, VmError> {
        Ok(Value::Unit)
    }
}

impl FromValue for () {
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            Value::Unit => Ok(()),
            actual => Err(VmError::ExpectedUnit {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}

macro_rules! impl_from_value_tuple {
    () => {
    };

    ({$ty:ident, $var:ident, $count:expr}, $({$l_ty:ident, $l_var:ident, $l_count:expr},)*) => {
        impl_from_value_tuple!{@impl $count, {$ty, $var, $count}, $({$l_ty, $l_var, $l_count},)*}
        impl_from_value_tuple!{$({$l_ty, $l_var, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $var:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> FromValue for ($($ty,)*)
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
                let slot = value.into_tuple(vm)?;
                let tuple = vm.tuple_clone(slot)?;

                if tuple.len() != $count {
                    return Err(VmError::ExpectedTupleLength {
                        actual: tuple.len(),
                        expected: $count,
                    });
                }

                #[allow(unused_mut, unused_variables)]
                let mut it = tuple.iter();

                $(
                    let $var: $ty = match it.next() {
                        Some(value) => <$ty>::from_value(*value, vm)?,
                        None => {
                            return Err(VmError::IterationError);
                        },
                    };
                )*

                Ok(($($var,)*))
            }
        }

        impl<$($ty,)*> ToValue for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            fn to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_value(vm)?;)*
                Ok(vm.tuple_allocate(vec![$($var,)*].into_boxed_slice()))
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
