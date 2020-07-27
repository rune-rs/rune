use crate::value::{self, Value, ValueRef, ValueType};
use crate::vm::Vm;
use thiserror::Error;

/// Failure to allocate a value.
#[derive(Debug, Error)]
#[error("failed to allocate value")]
pub struct AllocateError(());

/// Failure to encode a value.
#[derive(Debug, Error)]
#[error("failed to encode arguments")]
pub struct EncodeError(());

/// Trait for converting arguments into values.
pub trait IntoArgs {
    /// Encode arguments to the vm.
    fn encode(self, vm: &mut Vm) -> Result<(), EncodeError>;

    /// The number of arguments.
    fn count() -> usize;
}

/// Trait for converting types into values.
pub trait ReflectValueType: Sized {
    /// Convert into a value type.
    fn reflect_value_type() -> ValueType;
}

/// Trait used to allocate values through [allocate].
///
/// [allocate][crate::Vm::allocate].
pub trait Allocate {
    /// Allocate the given value into the vm.
    fn allocate(self, vm: &mut Vm) -> Result<ValueRef, AllocateError>;
}

/// Trait for converting types into values.
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self, vm: &mut Vm) -> Option<ValueRef>;
}

/// Trait for converting from a value.
pub trait FromValue: Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: ValueRef, vm: &Vm) -> Result<Self, ValueRef>;
}

impl<T> ReflectValueType for Option<T>
where
    T: ReflectValueType,
{
    fn reflect_value_type() -> ValueType {
        T::reflect_value_type()
    }
}

impl FromValue for Value {
    fn from_value(value: ValueRef, vm: &Vm) -> Result<Self, ValueRef> {
        Ok(vm.to_owned_value(value))
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: ValueRef, vm: &Vm) -> Result<Self, ValueRef> {
        let slot = value.into_slot::<value::ManagedArray>()?;

        let holder = match vm.arrays.get(slot) {
            Some(array) => array,
            None => return Err(value),
        };

        let mut output = Vec::with_capacity(holder.value.len());

        for value in holder.value.iter().copied() {
            output.push(T::from_value(value, vm)?);
        }

        Ok(output)
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: ValueRef, vm: &Vm) -> Result<Self, ValueRef> {
        match value {
            ValueRef::Unit => Ok(None),
            _ => Ok(Some(T::from_value(value, vm)?)),
        }
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Option<ValueRef> {
        match self {
            Some(value) => value.to_value(vm),
            None => Some(ValueRef::Unit),
        }
    }
}

/// Convert a unit into a value type.
impl ReflectValueType for () {
    fn reflect_value_type() -> ValueType {
        ValueType::Unit
    }
}

impl Allocate for () {
    fn allocate(self, _vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(ValueRef::Unit)
    }
}

impl ToValue for () {
    fn to_value(self, _vm: &mut Vm) -> Option<ValueRef> {
        Some(ValueRef::Unit)
    }
}

impl FromValue for () {
    fn from_value(value: ValueRef, _vm: &Vm) -> Result<Self, ValueRef> {
        match value {
            ValueRef::Unit => Ok(()),
            value => Err(value),
        }
    }
}

/// Convert a unit into a value type.
impl ReflectValueType for bool {
    fn reflect_value_type() -> ValueType {
        ValueType::Bool
    }
}

impl Allocate for bool {
    fn allocate(self, _vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(ValueRef::Bool(self))
    }
}

impl ToValue for bool {
    fn to_value(self, _vm: &mut Vm) -> Option<ValueRef> {
        Some(ValueRef::Bool(self))
    }
}

impl FromValue for bool {
    fn from_value(value: ValueRef, _vm: &Vm) -> Result<Self, ValueRef> {
        match value {
            ValueRef::Bool(value) => Ok(value),
            value => Err(value),
        }
    }
}

/// Convert a string into a value type.
impl ReflectValueType for String {
    fn reflect_value_type() -> ValueType {
        ValueType::String
    }
}

impl Allocate for String {
    fn allocate(self, vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(vm.allocate_string(self.into_boxed_str()))
    }
}

impl ToValue for String {
    fn to_value(self, vm: &mut Vm) -> Option<ValueRef> {
        Some(vm.allocate_string(self.into_boxed_str()))
    }
}

impl FromValue for String {
    fn from_value(value: ValueRef, vm: &Vm) -> Result<Self, ValueRef> {
        let slot = value.into_slot::<value::ManagedString>()?;

        match vm.string_clone(slot) {
            Some(value) => Ok(String::from(value)),
            None => Err(value),
        }
    }
}

/// Convert a string into a value type.
impl ReflectValueType for Box<str> {
    fn reflect_value_type() -> ValueType {
        ValueType::String
    }
}

impl Allocate for Box<str> {
    fn allocate(self, vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(vm.allocate_string(self))
    }
}

impl ToValue for Box<str> {
    fn to_value(self, vm: &mut Vm) -> Option<ValueRef> {
        Some(vm.allocate_string(self))
    }
}

impl FromValue for Box<str> {
    fn from_value(value: ValueRef, vm: &Vm) -> Result<Self, ValueRef> {
        let slot = value.into_slot::<value::ManagedString>()?;

        match vm.string_clone(slot) {
            Some(value) => Ok(value),
            _ => Err(value),
        }
    }
}

/// Convert a number into a value type.
impl ReflectValueType for i64 {
    fn reflect_value_type() -> ValueType {
        ValueType::Integer
    }
}

impl Allocate for i64 {
    fn allocate(self, _vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(ValueRef::Integer(self))
    }
}

impl ToValue for i64 {
    fn to_value(self, _vm: &mut Vm) -> Option<ValueRef> {
        Some(ValueRef::Integer(self))
    }
}

impl FromValue for i64 {
    fn from_value(value: ValueRef, _vm: &Vm) -> Result<Self, ValueRef> {
        match value {
            ValueRef::Integer(number) => Ok(number),
            value => Err(value),
        }
    }
}

macro_rules! number_value_trait {
    ($ty:ty) => {
        /// Convert a number into a value type.
        impl ReflectValueType for $ty {
            fn reflect_value_type() -> ValueType {
                ValueType::Integer
            }
        }

        impl Allocate for $ty {
            fn allocate(self, _vm: &mut Vm) -> Result<ValueRef, AllocateError> {
                use std::convert::TryInto as _;

                let number = self.try_into().map_err(|_| AllocateError(()))?;
                Ok(ValueRef::Integer(number))
            }
        }

        impl ToValue for $ty {
            fn to_value(self, _vm: &mut Vm) -> Option<ValueRef> {
                use std::convert::TryInto as _;

                Some(ValueRef::Integer(self.try_into().ok()?))
            }
        }

        impl FromValue for $ty {
            fn from_value(value: ValueRef, _vm: &Vm) -> Result<Self, ValueRef> {
                use std::convert::TryInto as _;

                match value {
                    ValueRef::Integer(number) => {
                        number.try_into().map_err(|_| ValueRef::Integer(number))
                    }
                    value => Err(value),
                }
            }
        }
    };
}

number_value_trait!(u8);
number_value_trait!(u32);
number_value_trait!(u64);
number_value_trait!(u128);

number_value_trait!(i8);
number_value_trait!(i32);
number_value_trait!(i128);

/// Convert a float into a value type.
impl ReflectValueType for f64 {
    fn reflect_value_type() -> ValueType {
        ValueType::Float
    }
}

impl Allocate for f64 {
    fn allocate(self, _vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(ValueRef::Float(self))
    }
}

impl ToValue for f64 {
    fn to_value(self, _vm: &mut Vm) -> Option<ValueRef> {
        Some(ValueRef::Float(self))
    }
}

impl FromValue for f64 {
    fn from_value(value: ValueRef, _vm: &Vm) -> Result<Self, ValueRef> {
        match value {
            ValueRef::Float(number) => Ok(number),
            value => Err(value),
        }
    }
}

/// Convert a float into a value type.
impl ReflectValueType for f32 {
    fn reflect_value_type() -> ValueType {
        ValueType::Float
    }
}

impl Allocate for f32 {
    fn allocate(self, _vm: &mut Vm) -> Result<ValueRef, AllocateError> {
        Ok(ValueRef::Float(self as f64))
    }
}

impl ToValue for f32 {
    fn to_value(self, _vm: &mut Vm) -> Option<ValueRef> {
        use std::convert::TryInto as _;
        Some(ValueRef::Float(self.try_into().ok()?))
    }
}

impl FromValue for f32 {
    fn from_value(value: ValueRef, _vm: &Vm) -> Result<Self, ValueRef> {
        match value {
            ValueRef::Float(number) => Ok(number as f32),
            value => Err(value),
        }
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
            fn encode(self, vm: &mut Vm) -> Result<(), EncodeError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_value(vm).ok_or_else(|| EncodeError(()))?;)*
                $(vm.managed_push($var);)*
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
