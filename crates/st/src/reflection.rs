use crate::external::External;
use crate::value::{Value, ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{Integer, Mut, Ref, StackError, Vm};

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

impl<'a> UnsafeFromValue for &'a str {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(Ref::unsafe_into_ref(vm.string_ref(slot)?).as_str())
    }
}

impl<'a> UnsafeFromValue for &'a String {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(Ref::unsafe_into_ref(vm.string_ref(slot)?))
    }
}

impl<'a> ReflectValueType for &'a String {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<'a> UnsafeFromValue for &'a mut String {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(Mut::unsafe_into_mut(vm.string_mut(slot)?))
    }
}

impl<'a> ReflectValueType for &'a mut String {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<T> ReflectValueType for Option<T>
where
    T: ReflectValueType,
{
    fn value_type() -> ValueType {
        T::value_type()
    }

    fn value_type_info() -> ValueTypeInfo {
        T::value_type_info()
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        match self {
            Some(s) => s.to_value(vm),
            None => Ok(ValuePtr::Unit),
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        match value {
            ValuePtr::Unit => Ok(None),
            _ => Ok(Some(T::from_value(value, vm)?)),
        }
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_array()?;
        let array = vm.array_take(slot)?;

        let mut output = Vec::with_capacity(array.len());

        for value in array.iter().copied() {
            output.push(T::from_value(value, vm)?);
        }

        Ok(output)
    }
}

/// Convert a unit into a value type.
impl ReflectValueType for () {
    fn value_type() -> ValueType {
        ValueType::Unit
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Unit
    }
}

impl ToValue for () {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(ValuePtr::Unit)
    }
}

impl FromValue for () {
    fn from_value(_: ValuePtr, _vm: &mut Vm) -> Result<Self, StackError> {
        Ok(())
    }
}

/// Convert a unit into a value type.
impl ReflectValueType for bool {
    fn value_type() -> ValueType {
        ValueType::Bool
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bool
    }
}

impl ToValue for bool {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(ValuePtr::Bool(self))
    }
}

impl FromValue for bool {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, StackError> {
        match value {
            ValuePtr::Bool(value) => Ok(value),
            _ => Err(StackError::ExpectedBoolean),
        }
    }
}

impl ReflectValueType for String {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<'a> ReflectValueType for &'a str {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl ToValue for String {
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(vm.allocate_string(self))
    }
}

impl FromValue for String {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        vm.string_take(slot)
    }
}

/// Convert a string into a value type.
impl ReflectValueType for Box<str> {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl ToValue for Box<str> {
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(vm.allocate_string(self.to_string()))
    }
}

impl FromValue for Box<str> {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(vm.string_take(slot)?.into_boxed_str())
    }
}

/// Convert a number into a value type.
impl ReflectValueType for i64 {
    fn value_type() -> ValueType {
        ValueType::Integer
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Integer
    }
}

impl ToValue for i64 {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(ValuePtr::Integer(self))
    }
}

impl FromValue for i64 {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, StackError> {
        match value {
            ValuePtr::Integer(number) => Ok(number),
            _ => Err(StackError::ExpectedInteger),
        }
    }
}

macro_rules! number_value_trait {
    ($ty:ty, $variant:ident) => {
        /// Convert a number into a value type.
        impl ReflectValueType for $ty {
            fn value_type() -> ValueType {
                ValueType::Integer
            }

            fn value_type_info() -> ValueTypeInfo {
                ValueTypeInfo::Integer
            }
        }

        impl ToValue for $ty {
            fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(ValuePtr::Integer(number)),
                    Err(..) => Err(StackError::IntegerToValueCoercionError {
                        from: Integer::$variant(self),
                        to: std::any::type_name::<i64>(),
                    }),
                }
            }
        }

        impl FromValue for $ty {
            fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, StackError> {
                use std::convert::TryInto as _;

                match value {
                    ValuePtr::Integer(number) => match number.try_into() {
                        Ok(number) => Ok(number),
                        Err(..) => Err(StackError::ValueToIntegerCoercionError {
                            from: Integer::I64(number),
                            to: std::any::type_name::<Self>(),
                        }),
                    },
                    _ => Err(StackError::ExpectedInteger),
                }
            }
        }
    };
}

number_value_trait!(u8, U8);
number_value_trait!(u32, U32);
number_value_trait!(u64, U64);
number_value_trait!(u128, U128);
number_value_trait!(usize, Usize);

number_value_trait!(i8, I8);
number_value_trait!(i32, I32);
number_value_trait!(i128, I128);
number_value_trait!(isize, Isize);

/// Convert a float into a value type.
impl ReflectValueType for f64 {
    fn value_type() -> ValueType {
        ValueType::Float
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Float
    }
}

impl ToValue for f64 {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(ValuePtr::Float(self))
    }
}

impl FromValue for f64 {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, StackError> {
        match value {
            ValuePtr::Float(number) => Ok(number),
            _ => Err(StackError::ExpectedFloat),
        }
    }
}

/// Convert a float into a value type.
impl ReflectValueType for f32 {
    fn value_type() -> ValueType {
        ValueType::Float
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Float
    }
}

impl ToValue for f32 {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(ValuePtr::Float(self as f64))
    }
}

impl FromValue for f32 {
    fn from_value(value: ValuePtr, _vm: &mut Vm) -> Result<Self, StackError> {
        match value {
            ValuePtr::Float(number) => Ok(number as f32),
            _ => Err(StackError::ExpectedFloat),
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
            fn into_args(self, vm: &mut Vm) -> Result<(), StackError> {
                let ($($var,)*) = self;
                $(let $var = $var.to_value(vm)?;)*
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
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        vm.take_owned_value(value)
    }
}

impl FromValue for Box<dyn External> {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_external()?;
        vm.external_take_dyn(slot)
    }
}
