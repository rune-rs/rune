use crate::value::{Value, ValueType};
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError>;
}

/// Trait for converting types into values.
pub trait ReflectToValue: ReflectValueType {
    /// Convert into a value.
    fn reflect_to_value(self, _state: &mut Vm) -> Option<Value>;
}

/// Trait for converting from a value.
pub trait ReflectFromValue: ReflectValueType {
    /// Try to convert to the given type, from the given value.
    fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value>;
}

impl<T> ReflectValueType for Option<T>
where
    T: ReflectValueType,
{
    fn reflect_value_type() -> ValueType {
        T::reflect_value_type()
    }
}

impl<T> ReflectFromValue for Option<T>
where
    T: ReflectFromValue,
{
    fn reflect_from_value(value: Value, vm: &Vm) -> Result<Self, Value> {
        match value {
            Value::Unit => Ok(None),
            _ => Ok(Some(T::reflect_from_value(value, vm)?)),
        }
    }
}

impl<T> ReflectToValue for Option<T>
where
    T: ReflectToValue,
{
    fn reflect_to_value(self, vm: &mut Vm) -> Option<Value> {
        match self {
            Some(value) => value.reflect_to_value(vm),
            None => Some(Value::Unit),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        Ok(vm.allocate_value(Value::Unit))
    }
}

impl ReflectToValue for () {
    fn reflect_to_value(self, _state: &mut Vm) -> Option<Value> {
        Some(Value::Unit)
    }
}

impl ReflectFromValue for () {
    fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value> {
        match value {
            Value::Unit => Ok(()),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        Ok(vm.allocate_value(Value::Bool(self)))
    }
}

impl ReflectToValue for bool {
    fn reflect_to_value(self, _state: &mut Vm) -> Option<Value> {
        Some(Value::Bool(self))
    }
}

impl ReflectFromValue for bool {
    fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value> {
        match value {
            Value::Bool(value) => Ok(value),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        let index = vm.allocate_string(self.into_boxed_str());
        Ok(vm.allocate_value(Value::String(index)))
    }
}

impl ReflectToValue for String {
    fn reflect_to_value(self, vm: &mut Vm) -> Option<Value> {
        Some(Value::String(vm.allocate_string(self.into_boxed_str())))
    }
}

impl ReflectFromValue for String {
    fn reflect_from_value(value: Value, vm: &Vm) -> Result<Self, Value> {
        match value {
            Value::String(index) => match vm.cloned_string(index) {
                Some(value) => Ok((&*value).to_owned()),
                None => return Err(Value::String(index)),
            },
            value => Err(value),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        let index = vm.allocate_string(self);
        Ok(vm.allocate_value(Value::String(index)))
    }
}

impl ReflectToValue for Box<str> {
    fn reflect_to_value(self, vm: &mut Vm) -> Option<Value> {
        Some(Value::String(vm.allocate_string(self)))
    }
}

impl ReflectFromValue for Box<str> {
    fn reflect_from_value(value: Value, vm: &Vm) -> Result<Self, Value> {
        match value {
            Value::String(index) => match vm.cloned_string(index) {
                Some(value) => Ok(value),
                None => return Err(Value::String(index)),
            },
            value => Err(value),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        Ok(vm.allocate_value(Value::Integer(self)))
    }
}

impl ReflectToValue for i64 {
    fn reflect_to_value(self, _state: &mut Vm) -> Option<Value> {
        Some(Value::Integer(self))
    }
}

impl ReflectFromValue for i64 {
    fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value> {
        match value {
            Value::Integer(number) => Ok(number),
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
            fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
                use std::convert::TryInto as _;

                let number = self.try_into().map_err(|_| AllocateError(()))?;
                Ok(vm.allocate_value(Value::Integer(number)))
            }
        }

        impl ReflectToValue for $ty {
            fn reflect_to_value(self, _state: &mut Vm) -> Option<Value> {
                use std::convert::TryInto as _;

                Some(Value::Integer(self.try_into().ok()?))
            }
        }

        impl ReflectFromValue for $ty {
            fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value> {
                use std::convert::TryInto as _;

                match value {
                    Value::Integer(number) => number.try_into().map_err(|_| Value::Integer(number)),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        Ok(vm.allocate_value(Value::Float(self)))
    }
}

impl ReflectToValue for f64 {
    fn reflect_to_value(self, _state: &mut Vm) -> Option<Value> {
        Some(Value::Float(self))
    }
}

impl ReflectFromValue for f64 {
    fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value> {
        match value {
            Value::Float(number) => Ok(number),
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
    fn allocate(self, vm: &mut Vm) -> Result<usize, AllocateError> {
        Ok(vm.allocate_value(Value::Float(self as f64)))
    }
}

impl ReflectToValue for f32 {
    fn reflect_to_value(self, _state: &mut Vm) -> Option<Value> {
        use std::convert::TryInto as _;
        Some(Value::Float(self.try_into().ok()?))
    }
}

impl ReflectFromValue for f32 {
    fn reflect_from_value(value: Value, _state: &Vm) -> Result<Self, Value> {
        match value {
            Value::Float(number) => Ok(number as f32),
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
            $($ty: ReflectToValue,)*
        {
            #[allow(unused)]
            fn encode(self, vm: &mut Vm) -> Result<(), EncodeError> {
                let ($($var,)*) = self;
                $(let $var = $var.reflect_to_value(vm).ok_or_else(|| EncodeError(()))?;)*
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
