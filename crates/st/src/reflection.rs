use crate::external::External;
use crate::value::{Value, ValuePtr, ValueType};
use crate::vm::Vm;
use thiserror::Error;

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

/// Trait for converting types into values.
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self, vm: &mut Vm) -> Option<ValuePtr>;
}

/// Trait for converting from a value.
pub trait FromValue: Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr>;
}

/// A potentially unsafe conversion for value conversion.
pub trait UnsafeFromValue: Sized {
    /// Convert the given reference using unsafe assumptions to a value.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr>;
}

impl<T> UnsafeFromValue for T
where
    T: FromValue,
{
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        T::from_value(value, vm)
    }
}

impl<'a> UnsafeFromValue for &'a str {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        let slot = value.into_string()?;

        if let Ok(value) = vm.string_ref(slot) {
            return Ok(std::mem::transmute(value));
        }

        Err(value)
    }
}

impl<T> ReflectValueType for Option<T>
where
    T: ReflectValueType,
{
    fn reflect_value_type() -> ValueType {
        T::reflect_value_type()
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Option<ValuePtr> {
        match self {
            Some(s) => s.to_value(vm),
            None => Some(ValuePtr::Unit),
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        Ok(match value {
            ValuePtr::Unit => None,
            value => Some(T::from_value(value, vm)?),
        })
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        let slot = value.into_array()?;

        let array = match vm.array_take(slot) {
            Some(array) => array,
            None => return Err(value),
        };

        let mut output = Vec::with_capacity(array.len());

        for value in array.iter().copied() {
            output.push(T::from_value(value, vm)?);
        }

        Ok(output)
    }
}

/// Convert a unit into a value type.
impl ReflectValueType for () {
    fn reflect_value_type() -> ValueType {
        ValueType::Unit
    }
}

impl ToValue for () {
    fn to_value(self, _vm: &mut Vm) -> Option<ValuePtr> {
        Some(ValuePtr::Unit)
    }
}

impl FromValue for () {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, ValuePtr> {
        match value {
            ValuePtr::Unit => Ok(()),
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

impl ToValue for bool {
    fn to_value(self, _vm: &mut Vm) -> Option<ValuePtr> {
        Some(ValuePtr::Bool(self))
    }
}

impl FromValue for bool {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, ValuePtr> {
        match value {
            ValuePtr::Bool(value) => Ok(value),
            value => Err(value),
        }
    }
}

impl ReflectValueType for String {
    fn reflect_value_type() -> ValueType {
        ValueType::String
    }
}

impl<'a> ReflectValueType for &'a str {
    fn reflect_value_type() -> ValueType {
        ValueType::String
    }
}

impl ToValue for String {
    fn to_value(self, vm: &mut Vm) -> Option<ValuePtr> {
        Some(vm.allocate_string(self.into_boxed_str()))
    }
}

impl FromValue for String {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        let slot = value.into_string()?;

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

impl ToValue for Box<str> {
    fn to_value(self, vm: &mut Vm) -> Option<ValuePtr> {
        Some(vm.allocate_string(self))
    }
}

impl FromValue for Box<str> {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        let slot = value.into_string()?;

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

impl ToValue for i64 {
    fn to_value(self, _vm: &mut Vm) -> Option<ValuePtr> {
        Some(ValuePtr::Integer(self))
    }
}

impl FromValue for i64 {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, ValuePtr> {
        match value {
            ValuePtr::Integer(number) => Ok(number),
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

        impl ToValue for $ty {
            fn to_value(self, _vm: &mut Vm) -> Option<ValuePtr> {
                use std::convert::TryInto as _;

                Some(ValuePtr::Integer(self.try_into().ok()?))
            }
        }

        impl FromValue for $ty {
            fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, ValuePtr> {
                use std::convert::TryInto as _;

                match value {
                    ValuePtr::Integer(number) => {
                        number.try_into().map_err(|_| ValuePtr::Integer(number))
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
number_value_trait!(usize);

number_value_trait!(i8);
number_value_trait!(i32);
number_value_trait!(i128);
number_value_trait!(isize);

/// Convert a float into a value type.
impl ReflectValueType for f64 {
    fn reflect_value_type() -> ValueType {
        ValueType::Float
    }
}

impl ToValue for f64 {
    fn to_value(self, _vm: &mut Vm) -> Option<ValuePtr> {
        Some(ValuePtr::Float(self))
    }
}

impl FromValue for f64 {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, ValuePtr> {
        match value {
            ValuePtr::Float(number) => Ok(number),
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

impl ToValue for f32 {
    fn to_value(self, _vm: &mut Vm) -> Option<ValuePtr> {
        use std::convert::TryInto as _;
        Some(ValuePtr::Float(self.try_into().ok()?))
    }
}

impl FromValue for f32 {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, ValuePtr> {
        match value {
            ValuePtr::Float(number) => Ok(number as f32),
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

impl FromValue for Value {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        Ok(vm.take_owned_value(value))
    }
}

impl FromValue for Box<dyn External> {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, ValuePtr> {
        let slot = value.into_external()?;

        match vm.external_take_dyn(slot) {
            Some(external) => Ok(external),
            None => Err(value),
        }
    }
}
