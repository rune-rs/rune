use crate::{Any, AnyObj, Mut, RawMut, RawRef, Ref, Shared, Value, VmError};

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
        Ok(value.into_any()?)
    }
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

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(Ref::into_raw(value.into_option()?.into_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Option<Value> {
    type Output = *mut Option<Value>;
    type Guard = RawMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(Mut::into_raw(value.into_option()?.into_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

// String impls

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self, VmError> {
        match value {
            Value::String(string) => Ok(string.borrow_ref()?.clone()),
            Value::StaticString(string) => Ok((**string).clone()),
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

impl UnsafeFromValue for &str {
    type Output = *const str;
    type Guard = Option<RawRef>;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_ref()?;
                let (s, guard) = Ref::into_raw(string);
                ((*s).as_str(), Some(guard))
            }
            Value::StaticString(string) => (string.as_ref().as_str(), None),
            actual => return Err(VmError::expected::<String>(actual.type_info()?)),
        })
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut str {
    type Output = *mut str;
    type Guard = Option<RawMut>;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_mut()?;
                let (s, guard) = Mut::into_raw(string);
                ((*s).as_mut_str(), Some(guard))
            }
            actual => {
                return Err(VmError::expected::<String>(actual.type_info()?));
            }
        })
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl UnsafeFromValue for &String {
    type Output = *const String;
    type Guard = Option<RawRef>;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.into_ref()?;
                let (s, guard) = Ref::into_raw(string);
                (s, Some(guard))
            }
            Value::StaticString(string) => (&**string, None),
            actual => {
                return Err(VmError::expected::<String>(actual.type_info()?));
            }
        })
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut String {
    type Output = *mut String;
    type Guard = RawMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
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

    unsafe fn to_arg(output: Self::Output) -> Self {
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

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let result = value.into_result()?;
        let result = result.into_ref()?;
        Ok(Ref::into_raw(result))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

// Vec impls

impl FromValue for Mut<Vec<Value>> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_vec()?.into_mut()?)
    }
}

impl FromValue for Ref<Vec<Value>> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_vec()?.into_ref()?)
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        let vec = value.into_vec()?;
        let vec = vec.take()?;

        let mut output = Vec::with_capacity(vec.len());

        for value in vec {
            output.push(T::from_value(value)?);
        }

        Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a [Value] {
    type Output = *const [Value];
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        let (vec, guard) = Ref::into_raw(vec.into_ref()?);
        Ok((&**vec, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a Vec<Value> {
    type Output = *const Vec<Value>;
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(Ref::into_raw(vec.into_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Vec<Value> {
    type Output = *mut Vec<Value>;
    type Guard = RawMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(Mut::into_raw(vec.into_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

// number impls

impl FromValue for () {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_unit()?)
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_byte()?)
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_bool()?)
    }
}

impl FromValue for char {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_char()?)
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_integer()?)
    }
}

macro_rules! impl_number {
    ($ty:ty, $variant:ident) => {
        impl FromValue for $ty {
            fn from_value(value: Value) -> Result<Self, VmError> {
                use std::convert::TryInto as _;
                let integer = value.into_integer()?;

                match integer.try_into() {
                    Ok(number) => Ok(number),
                    Err(..) => Err($crate::VmError::from(
                        $crate::VmErrorKind::ValueToIntegerCoercionError {
                            from: $crate::VmIntegerRepr::I64(integer),
                            to: std::any::type_name::<Self>(),
                        },
                    )),
                }
            }
        }
    };
}

impl_number!(u32, U32);
impl_number!(u64, U64);
impl_number!(u128, U128);
impl_number!(usize, Usize);
impl_number!(i8, I8);
impl_number!(i32, I32);
impl_number!(i128, I128);
impl_number!(isize, Isize);

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_float()?)
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
