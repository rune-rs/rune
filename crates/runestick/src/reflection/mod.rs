use crate::{Any, OwnedMut, OwnedRef, Shared, Type, TypeInfo, Value, VmError};

mod bytes;
mod hash_map;
mod option;
mod primitive;
mod result;
mod string;
mod tuple;
mod vec;

/// Trait for converting types into values.
pub trait ValueType {
    /// Convert into a value type.
    fn value_type() -> Type;

    /// Access diagnostical information on the value type.
    fn type_info() -> TypeInfo;
}

/// Blanket implementation for references.
impl<T: ?Sized> ValueType for &T
where
    T: ValueType,
{
    fn value_type() -> Type {
        T::value_type()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for mutable references.
impl<T: ?Sized> ValueType for &mut T
where
    T: ValueType,
{
    fn value_type() -> Type {
        T::value_type()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Trait for converting types into values.
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self) -> Result<Value, VmError>;
}

/// Trait for converting from a value.
pub trait FromValue: 'static + Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: Value) -> Result<Self, VmError>;
}

/// Helper trait to convert from the internal any type.
pub trait FromAny: 'static + Sized {
    /// Try to convert from the internal any type.
    fn from_any(any: Shared<Any>) -> Result<Self, VmError>;
}

impl<T> FromAny for OwnedRef<T>
where
    T: std::any::Any,
{
    fn from_any(any: Shared<Any>) -> Result<Self, VmError> {
        Ok(any.downcast_owned_ref()?)
    }
}

impl<T> FromAny for OwnedMut<T>
where
    T: std::any::Any,
{
    fn from_any(any: Shared<Any>) -> Result<Self, VmError> {
        Ok(any.downcast_owned_mut()?)
    }
}

/// A potentially unsafe conversion for value conversion.
///
/// This trait is used to convert values to references, which can be safely used
/// while an external function call is used. That sort of use is safe because we
/// hold onto the guard returned by the conversion during external function
/// calls.
pub trait UnsafeFromValue: Sized {
    /// The output type from the unsafe coercion.
    type Output: 'static;

    /// The raw guard returned.
    ///
    /// Must only be dropped *after* the value returned from this function is
    /// no longer live.
    type Guard: 'static;

    /// Convert the given reference using unsafe assumptions to a value.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    ///
    /// You must also make sure that the returned value does not outlive the
    /// guard.
    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError>;

    /// Coerce the output of an unsafe from value into the final output type.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    ///
    /// You must also make sure that the returned value does not outlive the
    /// guard.
    unsafe fn to_arg(output: Self::Output) -> Self;
}

impl<T> UnsafeFromValue for T
where
    T: FromValue,
{
    type Output = T;
    type Guard = ();

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self, Self::Guard), VmError> {
        Ok((T::from_value(value)?, ()))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        output
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value)
    }
}

impl<T> ToValue for T
where
    Value: From<T>,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(self))
    }
}

impl ToValue for &Value {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(self.clone())
    }
}
