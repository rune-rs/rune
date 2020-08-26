use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::shared::Shared;
use crate::value::{Value, ValueError, ValueType, ValueTypeInfo};

macro_rules! impl_map {
    ($($tt:tt)*) => {
        impl<T> ReflectValueType for $($tt)*<String, T> {
            type Owned = $($tt)*<String, T>;

            fn value_type() -> ValueType {
                ValueType::Object
            }

            fn value_type_info() -> ValueTypeInfo {
                ValueTypeInfo::Object
            }
        }

        impl<T> FromValue for $($tt)*<String, T>
        where
            T: FromValue,
        {
            fn from_value(value: Value) -> Result<Self, ValueError> {
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
            fn to_value(self) -> Result<Value, ValueError> {
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
