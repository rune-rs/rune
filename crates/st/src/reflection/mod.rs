use crate::external::External;
use crate::value::{Value, ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{StackError, Vm};

mod hash_map;
mod option;
mod primitive;
mod string;
mod vec;

/// Trait for converting arguments into values.
pub trait IntoArgs {
    /// Encode arguments to the vm.
    fn into_args(self, vm: &mut Vm) -> Result<(), StackError>;

    /// The number of arguments.
    fn count() -> usize;
}

/// Trait for converting types into values.
pub trait ReflectValueType: Sized {
    /// Convert into a value type.
    fn value_type() -> ValueType;

    /// Access diagnostical information on the value type.
    fn value_type_info() -> ValueTypeInfo;
}

/// Trait for converting types into values.
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError>;
}

/// Trait for converting from a value.
pub trait FromValue: Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError>;
}

/// A potentially unsafe conversion for value conversion.
pub trait UnsafeFromValue: Sized {
    /// Convert the given reference using unsafe assumptions to a value.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError>;
}

impl<T> UnsafeFromValue for T
where
    T: FromValue,
{
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        T::from_value(value, vm)
    }
}

impl FromValue for ValuePtr {
    fn from_value(value: ValuePtr, _: &mut Vm) -> Result<Self, StackError> {
        Ok(value)
    }
}

impl ToValue for ValuePtr {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(self)
    }
}

impl FromValue for Value {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        vm.value_take(value)
    }
}

impl FromValue for Box<dyn External> {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_external(vm)?;
        vm.external_take_dyn(slot)
    }
}

macro_rules! impl_into_args {
    () => {
        impl_into_args!{@impl 0,}
    };

    ({$ty:ident, $var:ident, $count:expr}, $({$l_ty:ident, $l_var:ident, $l_count:expr},)*) => {
        impl_into_args!{@impl $count, {$ty, $var, $count}, $({$l_ty, $l_var, $l_count},)*}
        impl_into_args!{$({$l_ty, $l_var, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $var:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> IntoArgs for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            #[allow(unused)]
            fn into_args(self, vm: &mut Vm) -> Result<(), StackError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_value(vm)?;)*
                $(vm.unmanaged_push($var);)*
                Ok(())
            }

            fn count() -> usize {
                $count
            }
        }
    };
}

impl_into_args!(
    {H, h, 8},
    {G, g, 7},
    {F, f, 6},
    {E, e, 5},
    {D, d, 4},
    {C, c, 3},
    {B, b, 2},
    {A, a, 1},
);
