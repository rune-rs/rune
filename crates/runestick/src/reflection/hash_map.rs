macro_rules! impl_map {
    ($ty:ty) => {
        impl_static_type!(impl<T> $ty => crate::OBJECT_TYPE);

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

        impl<T> $crate::ToValue for $ty
        where
            T: $crate::ToValue,
        {
            fn to_value(self) -> Result<$crate::Value, $crate::VmError> {
                let mut output = crate::collections::HashMap::with_capacity(self.len());

                for (key, value) in self {
                    output.insert(key, value.to_value()?);
                }

                Ok($crate::Value::from($crate::Shared::new(output)))
            }
        }
    }
}

impl_map!(std::collections::HashMap<String, T>);
