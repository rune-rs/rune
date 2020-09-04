use crate::{
    FromValue, Object, OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, Shared, ToValue,
    UnsafeFromValue, Value, VmError,
};

value_types!(impl crate::OBJECT_TYPE, Object<T> => T Object<T>, T &Object<T>, T &mut Object<T>);

impl<T> FromValue for Object<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        let object = value.into_object()?;
        let object = object.take()?;
        let mut output = Object::with_capacity(object.len());

        for (key, value) in object {
            output.insert(key, T::from_value(value)?);
        }

        Ok(output)
    }
}

impl UnsafeFromValue for &Object<Value> {
    type Output = *const Object<Value>;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let object = value.into_object()?;
        let object = object.owned_ref()?;
        Ok(OwnedRef::into_raw(object))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Object<Value> {
    type Output = *mut Object<Value>;
    type Guard = RawOwnedMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let object = value.into_object()?;
        let object = object.owned_mut()?;
        Ok(OwnedMut::into_raw(object))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<T> ToValue for Object<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let mut object = Object::with_capacity(self.len());

        for (key, value) in self {
            object.insert(key, value.to_value()?);
        }

        Ok(Value::Object(Shared::new(object)))
    }
}
