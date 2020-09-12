use crate::{Any, AnyObj, Panic, Shared, Value, VmError, VmErrorKind};

/// Trait for converting types into values.
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self) -> Result<Value, VmError>;
}

/// Trait for converting types into values.
pub trait UnsafeToValue: Sized {
    /// The type used to guard the unsafe value conversion.
    type Guard: 'static;

    /// Convert into a value.
    unsafe fn unsafe_to_value(self) -> Result<(Value, Self::Guard), VmError>;
}

impl<T> ToValue for T
where
    T: Any,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(AnyObj::new(self)))
    }
}

impl<T> UnsafeToValue for T
where
    T: ToValue,
{
    type Guard = ();

    unsafe fn unsafe_to_value(self) -> Result<(Value, Self::Guard), VmError> {
        Ok((self.to_value()?, ()))
    }
}

impl ToValue for &Value {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(self.clone())
    }
}

// Option impls

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(Shared::new(match self {
            Some(some) => {
                let value = some.to_value()?;
                Some(value)
            }
            None => None,
        })))
    }
}

// String impls

impl ToValue for Box<str> {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(Shared::new(self.to_string())))
    }
}

// Result impls

impl<T> ToValue for Result<T, Panic>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(value) => Ok(value.to_value()?),
            Err(reason) => Err(VmError::from(VmErrorKind::Panic { reason })),
        }
    }
}

impl<T> ToValue for Result<T, VmError>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(value) => Ok(value.to_value()?),
            Err(error) => Err(error),
        }
    }
}

impl<T, E> ToValue for Result<T, E>
where
    T: ToValue,
    E: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(match self {
            Ok(ok) => {
                let ok = ok.to_value()?;
                Value::from(Shared::new(Ok(ok)))
            }
            Err(err) => {
                let err = err.to_value()?;
                Value::from(Shared::new(Err(err)))
            }
        })
    }
}

// Vec impls

impl<T> ToValue for Vec<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let mut vec = Vec::with_capacity(self.len());

        for value in self {
            vec.push(value.to_value()?);
        }

        Ok(Value::from(Shared::new(vec)))
    }
}

// number impls

macro_rules! number_value_trait {
    ($ty:ty, $variant:ident) => {
        impl ToValue for $ty {
            fn to_value(self) -> Result<Value, $crate::VmError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(Value::Integer(number)),
                    Err(..) => Err($crate::VmError::from(
                        $crate::VmErrorKind::IntegerToValueCoercionError {
                            from: $crate::VmIntegerRepr::$variant(self),
                            to: std::any::type_name::<i64>(),
                        },
                    )),
                }
            }
        }
    };
}

number_value_trait!(u32, U32);
number_value_trait!(u64, U64);
number_value_trait!(u128, U128);
number_value_trait!(usize, Usize);
number_value_trait!(i8, I8);
number_value_trait!(i32, I32);
number_value_trait!(i128, I128);
number_value_trait!(isize, Isize);

impl ToValue for f32 {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::Float(self as f64))
    }
}

// map impls

macro_rules! impl_map {
    ($ty:ty) => {
        impl<T> $crate::ToValue for $ty
        where
            T: $crate::ToValue,
        {
            fn to_value(self) -> Result<$crate::Value, $crate::VmError> {
                let mut output = crate::Object::with_capacity(self.len());

                for (key, value) in self {
                    output.insert(key, value.to_value()?);
                }

                Ok($crate::Value::from($crate::Shared::new(output)))
            }
        }
    };
}

impl_map!(std::collections::HashMap<String, T>);
