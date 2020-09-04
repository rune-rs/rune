use crate::{
    FromValue, OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, Shared, ToValue, UnsafeFromValue,
    Value, VmError,
};

value_types!(impl crate::VEC_TYPE, Vec<T> => T Vec<T>, T &Vec<T>, T &mut Vec<T>);
value_types!(crate::VEC_TYPE, Vec<Value> => &[Value]);

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
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        let (vec, guard) = OwnedRef::into_raw(vec.owned_ref()?);
        Ok((&**vec, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a Vec<Value> {
    type Output = *const Vec<Value>;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(OwnedRef::into_raw(vec.owned_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Vec<Value> {
    type Output = *mut Vec<Value>;
    type Guard = RawOwnedMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(OwnedMut::into_raw(vec.owned_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<T> ToValue for Vec<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let mut vec = Vec::with_capacity(self.len());

        for value in self {
            vec.push(value.to_value()?);
        }

        Ok(Value::Vec(Shared::new(vec)))
    }
}
