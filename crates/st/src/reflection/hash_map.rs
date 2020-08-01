use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{StackError, Vm};

macro_rules! impl_map {
    ($($tt:tt)*) => {
        impl<T> ReflectValueType for $($tt)*<String, T> {
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
            fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
                let slot = value.into_array(vm)?;
                let object = vm.object_take(slot)?;

                let mut output = $($tt)*::with_capacity(object.len());

                for (key, value) in object {
                    output.insert(key, T::from_value(value, vm)?);
                }

                Ok(output)
            }
        }

        impl<T> ToValue for $($tt)*<String, T>
        where
            T: ToValue,
        {
            fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
                let mut object = crate::collections::HashMap::with_capacity(self.len());

                for (key, value) in self {
                    object.insert(key, value.to_value(vm)?);
                }

                Ok(vm.object_allocate(object))
            }
        }
    }
}

impl_map!(std::collections::HashMap);
