use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::shared::{OwnRef, RawOwnRef, Shared};
use crate::value::{Object, Value, ValueError, ValueType, ValueTypeInfo};

impl<T> ReflectValueType for Object<T> {
    type Owned = Object<T>;

    fn value_type() -> ValueType {
        ValueType::Object
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Object
    }
}

impl<'a, T> ReflectValueType for &'a Object<T> {
    type Owned = Object<T>;

    fn value_type() -> ValueType {
        ValueType::Object
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Object
    }
}

impl<'a, T> ReflectValueType for &'a mut Object<T> {
    type Owned = Object<T>;

    fn value_type() -> ValueType {
        ValueType::Object
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Object
    }
}

impl<T> FromValue for Object<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, ValueError> {
        let object = value.into_object()?;
        let object = object.take()?;
        let mut output = Object::with_capacity(object.len());

        for (key, value) in object {
            output.insert(key, T::from_value(value)?);
        }

        Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a Object<Value> {
    type Output = *const Object<Value>;
    type Guard = RawOwnRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let object = value.into_object()?;
        let object = object.own_ref()?;
        Ok(OwnRef::into_raw(object))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<T> ToValue for Object<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, ValueError> {
        let mut object = Object::with_capacity(self.len());

        for (key, value) in self {
            object.insert(key, value.to_value()?);
        }

        Ok(Value::Object(Shared::new(object)))
    }
}
