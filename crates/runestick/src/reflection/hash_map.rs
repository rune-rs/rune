use crate::{FromValue, Shared, ToValue, Value, VmError};

macro_rules! impl_map {
    ($($tt:tt)*) => {
        value_types!(impl crate::OBJECT_TYPE, $($tt)*<String, T> => T $($tt)*<String, T>);

        impl<T> FromValue for $($tt)*<String, T>
        where
            T: FromValue,
        {
            fn from_value(value: Value) -> Result<Self, VmError> {
                let object = value.into_object()?;
                let object = object.take()?;

                let mut output = $($tt)*::with_capacity(object.len());

                for (key, value) in object {
                    output.insert(key, T::from_value(value)?);
                }

                Ok(output)
            }
        }

        impl<T> ToValue for $($tt)*<String, T>
        where
            T: ToValue,
        {
            fn to_value(self) -> Result<Value, VmError> {
                let mut output = crate::collections::HashMap::with_capacity(self.len());

                for (key, value) in self {
                    output.insert(key, value.to_value()?);
                }

                Ok(Value::Object(Shared::new(output)))
            }
        }
    }
}

impl_map!(std::collections::HashMap);
