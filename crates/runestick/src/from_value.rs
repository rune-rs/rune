use crate::{Any, AnyObj, Mut, RawMut, RawRef, Ref, Shared, StaticString, Value, VmError};
use std::sync::Arc;

/// Trait for converting from a value.
pub trait FromValue: 'static + Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: Value) -> Result<Self, VmError>;
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
    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError>;

    /// Coerce the output of an unsafe from value into the final output type.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    ///
    /// You must also make sure that the returned value does not outlive the
    /// guard.
    unsafe fn unsafe_coerce(output: Self::Output) -> Self;
}

impl<T> FromValue for T
where
    T: Any,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_any()?.take_downcast()?)
    }
}

impl<T> FromValue for Mut<T>
where
    T: Any,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_any()?.downcast_into_mut()?)
    }
}

impl<T> FromValue for Ref<T>
where
    T: Any,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_any()?.downcast_into_ref()?)
    }
}

impl FromValue for Shared<AnyObj> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_any()
    }
}

impl<T> UnsafeFromValue for T
where
    T: FromValue,
{
    type Output = T;
    type Guard = ();

    fn from_value(value: Value) -> Result<(Self, Self::Guard), VmError> {
        Ok((T::from_value(value)?, ()))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        output
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value)
    }
}

// Option impls

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(match value.into_option()?.take()? {
            Some(some) => Some(T::from_value(some)?),
            None => None,
        })
    }
}

impl UnsafeFromValue for &Option<Value> {
    type Output = *const Option<Value>;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(Ref::into_raw(value.into_option()?.into_ref()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Option<Value> {
    type Output = *mut Option<Value>;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(Mut::into_raw(value.into_option()?.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl UnsafeFromValue for &mut Result<Value, Value> {
    type Output = *mut Result<Value, Value>;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(Mut::into_raw(value.into_result()?.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

// String impls

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self, VmError> {
        match value {
            Value::String(string) => Ok(string.borrow_ref()?.clone()),
            Value::StaticString(string) => Ok((**string).to_owned()),
            actual => Err(VmError::expected::<String>(actual.type_info()?)),
        }
    }
}

impl FromValue for Mut<String> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        match value {
            Value::String(string) => Ok(string.into_mut()?),
            actual => Err(VmError::expected::<String>(actual.type_info()?)),
        }
    }
}

impl FromValue for Ref<String> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        match value {
            Value::String(string) => Ok(string.into_ref()?),
            actual => Err(VmError::expected::<String>(actual.type_info()?)),
        }
    }
}

impl FromValue for Box<str> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let string = value.into_string()?;
        let string = string.borrow_ref()?.clone();
        Ok(string.into_boxed_str())
    }
}

/// Raw guard used for `&str` references.
///
/// Note that we need to hold onto an instance of the static string to prevent
/// the reference to it from being deallocated (the `StaticString` variant).
pub enum StrGuard {
    RawRef(RawRef),
    StaticString(Arc<StaticString>),
}

impl UnsafeFromValue for &str {
    type Output = *const str;
    type Guard = StrGuard;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_ref()?;
                let (s, guard) = Ref::into_raw(string);
                // Safety: we're holding onto the guard for the string here, so
                // it is live.
                (unsafe { (*s).as_str() }, StrGuard::RawRef(guard))
            }
            Value::StaticString(string) => {
                (string.as_ref().as_str(), StrGuard::StaticString(string))
            }
            actual => return Err(VmError::expected::<String>(actual.type_info()?)),
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut str {
    type Output = *mut str;
    type Guard = Option<RawMut>;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_mut()?;
                let (s, guard) = Mut::into_raw(string);
                // Safety: we're holding onto the guard for the string here, so
                // it is live.
                (unsafe { (*s).as_mut_str() }, Some(guard))
            }
            actual => {
                return Err(VmError::expected::<String>(actual.type_info()?));
            }
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl UnsafeFromValue for &String {
    type Output = *const String;
    type Guard = StrGuard;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_ref()?;
                let (s, guard) = Ref::into_raw(string);
                (s, StrGuard::RawRef(guard))
            }
            Value::StaticString(string) => (&**string, StrGuard::StaticString(string)),
            actual => {
                return Err(VmError::expected::<String>(actual.type_info()?));
            }
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut String {
    type Output = *mut String;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_mut()?;
                let (s, guard) = Mut::into_raw(string);
                (s, guard)
            }
            actual => {
                return Err(VmError::expected::<String>(actual.type_info()?));
            }
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

// Result impls

impl<T, E> FromValue for Result<T, E>
where
    T: FromValue,
    E: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(match value.into_result()?.take()? {
            Ok(ok) => Ok(T::from_value(ok)?),
            Err(err) => Err(E::from_value(err)?),
        })
    }
}

impl UnsafeFromValue for &Result<Value, Value> {
    type Output = *const Result<Value, Value>;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let result = value.into_result()?;
        let result = result.into_ref()?;
        Ok(Ref::into_raw(result))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

// number impls

impl FromValue for () {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_unit()
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_byte()
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_bool()
    }
}

impl FromValue for char {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_char()
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_integer()
    }
}

macro_rules! impl_number {
    ($ty:ty) => {
        impl FromValue for $ty {
            fn from_value(value: Value) -> Result<Self, VmError> {
                use std::convert::TryInto as _;
                let integer = value.into_integer()?;

                match integer.try_into() {
                    Ok(number) => Ok(number),
                    Err(..) => Err($crate::VmError::from(
                        $crate::VmErrorKind::ValueToIntegerCoercionError {
                            from: $crate::VmIntegerRepr::from(integer),
                            to: std::any::type_name::<Self>(),
                        },
                    )),
                }
            }
        }
    };
}

impl_number!(u16);
impl_number!(u32);
impl_number!(u64);
impl_number!(u128);
impl_number!(usize);
impl_number!(i8);
impl_number!(i16);
impl_number!(i32);
impl_number!(i128);
impl_number!(isize);

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_float()
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_float()? as f32)
    }
}

// map impls

macro_rules! impl_map {
    ($ty:ty) => {
        impl<T> $crate::FromValue for $ty
        where
            T: $crate::FromValue,
        {
            fn from_value(value: $crate::Value) -> Result<Self, $crate::VmError> {
                let object = value.into_object()?;
                let object = object.take()?;

                let mut output = <$ty>::with_capacity(object.len());

                for (key, value) in object {
                    output.insert(key, T::from_value(value)?);
                }

                Ok(output)
            }
        }
    };
}

impl_map!(std::collections::HashMap<String, T>);
