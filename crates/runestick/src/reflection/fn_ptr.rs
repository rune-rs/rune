use crate::{
    FnPtr, FromValue, OwnedRef, RawOwnedRef, Shared, ToValue, UnsafeFromValue, Value, ValueError,
};

value_types!(crate::FN_PTR_TYPE, FnPtr => FnPtr, &FnPtr, Shared<FnPtr>, OwnedRef<FnPtr>);

impl FromValue for FnPtr {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_fn_ptr()?.take()?)
    }
}

impl FromValue for Shared<FnPtr> {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_fn_ptr()?)
    }
}

impl FromValue for OwnedRef<FnPtr> {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_fn_ptr()?.owned_ref()?)
    }
}

impl UnsafeFromValue for &FnPtr {
    type Output = *const FnPtr;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let fn_ptr = value.into_fn_ptr()?;
        let (fn_ptr, guard) = OwnedRef::into_raw(fn_ptr.owned_ref()?);
        Ok((fn_ptr, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl ToValue for FnPtr {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::FnPtr(Shared::new(self)))
    }
}

impl ToValue for Shared<FnPtr> {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::FnPtr(self))
    }
}
