macro_rules! inline_into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $as:ident,
        $as_mut:ident,
    ) => {
        $(#[$($meta)*])*
        ///
        /// This gets a copy of the value.
        #[inline]
        pub fn $as(&self) -> Result<$ty, RuntimeError> {
            match &self.repr {
                Repr::Inline(Inline::$kind(value)) => {
                    Ok(*value)
                }
                Repr::Inline(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                Repr::Dynamic(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                Repr::Any(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                Repr::Empty => {
                    Err(RuntimeError::from(AccessError::empty()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This gets the value by mutable reference.
        #[inline]
        pub fn $as_mut(&mut self) -> Result<&mut $ty, RuntimeError> {
            match &mut self.repr {
                Repr::Inline(Inline::$kind(value)) => {
                    Ok(value)
                }
                Repr::Inline(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                Repr::Dynamic(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                Repr::Any(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                Repr::Empty => {
                    Err(RuntimeError::from(AccessError::empty()))
                }
            }
        }
    }
}

macro_rules! any_from {
    ($($ty:ty),* $(,)*) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = alloc::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    Value::new(value)
                }
            }

            impl IntoOutput for $ty {
                type Output = $ty;

                #[inline]
                fn into_output(self) -> VmResult<Self::Output> {
                    VmResult::Ok(self)
                }
            }
        )*
    };
}

macro_rules! inline_from {
    ($($variant:ident => $ty:ty),* $(,)*) => {
        $(
            impl From<$ty> for $crate::runtime::Value {
                #[inline]
                fn from(value: $ty) -> Self {
                    $crate::runtime::Value::from($crate::runtime::Inline::$variant(value))
                }
            }

            impl From<$ty> for $crate::runtime::ConstValue {
                #[inline]
                fn from(value: $ty) -> Self {
                    $crate::runtime::ConstValue::from($crate::runtime::Inline::$variant(value))
                }
            }

            impl $crate::runtime::IntoOutput for $ty {
                type Output = $ty;

                #[inline]
                fn into_output(self) -> $crate::runtime::VmResult<Self::Output> {
                    $crate::runtime::VmResult::Ok(self)
                }
            }

            impl $crate::runtime::ToValue for $ty {
                #[inline]
                fn to_value(self) -> Result<Value, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::Value::from(self))
                }
            }

            impl $crate::runtime::ToConstValue for $ty {
                #[inline]
                fn to_const_value(self) -> Result<$crate::runtime::ConstValue, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::ConstValue::from(self))
                }
            }
        )*
    };
}

macro_rules! signed_value_trait {
    ($($ty:ty),* $(,)?) => {
        $(
            #[allow(clippy::needless_question_mark)]
            impl $crate::runtime::ToValue for $ty {
                #[inline]
                fn to_value(self) -> Result<Value, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::Value::try_from(self)?)
                }
            }

            #[allow(clippy::needless_question_mark)]
            impl $crate::runtime::ToConstValue for $ty {
                #[inline]
                fn to_const_value(self) -> Result<$crate::runtime::ConstValue, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::ConstValue::try_from(self)?)
                }
            }
        )*
    };
}

macro_rules! signed_value_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for $crate::runtime::Value {
                #[inline]
                fn from(number: $ty) -> Self {
                    $crate::runtime::Value::from(number as i64)
                }
            }

            impl From<$ty> for $crate::runtime::ConstValue {
                #[inline]
                fn from(number: $ty) -> Self {
                    $crate::runtime::ConstValue::from(number as i64)
                }
            }
        )*
    }
}

macro_rules! signed_value_try_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = $crate::runtime::RuntimeError;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, $crate::runtime::RuntimeError> {
                    match <i64>::try_from(value) {
                        Ok(number) => Ok(Value::from(number)),
                        #[allow(unreachable_patterns)]
                        Err(..) => Err($crate::runtime::RuntimeError::from(VmErrorKind::IntegerToValueCoercionError {
                            from: VmIntegerRepr::from(value),
                            to: any::type_name::<i64>(),
                        })),
                    }
                }
            }

            impl TryFrom<$ty> for ConstValue {
                type Error = $crate::runtime::RuntimeError;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, $crate::runtime::RuntimeError> {
                    match <i64>::try_from(value) {
                        Ok(number) => Ok($crate::runtime::ConstValue::from(number)),
                        #[allow(unreachable_patterns)]
                        Err(..) => Err($crate::runtime::RuntimeError::from(VmErrorKind::IntegerToValueCoercionError {
                            from: VmIntegerRepr::from(value),
                            to: any::type_name::<i64>(),
                        })),
                    }
                }
            }
        )*
    }
}

macro_rules! unsigned_value_trait {
    ($($ty:ty),* $(,)?) => {
        $(
            #[allow(clippy::needless_question_mark)]
            impl $crate::runtime::ToValue for $ty {
                #[inline]
                fn to_value(self) -> Result<Value, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::Value::try_from(self)?)
                }
            }

            #[allow(clippy::needless_question_mark)]
            impl $crate::runtime::ToConstValue for $ty {
                #[inline]
                fn to_const_value(self) -> Result<$crate::runtime::ConstValue, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::ConstValue::try_from(self)?)
                }
            }
        )*
    };
}

macro_rules! unsigned_value_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Value {
                #[inline]
                fn from(number: $ty) -> Self {
                    Value::from(number as u64)
                }
            }

            impl From<$ty> for ConstValue {
                #[inline]
                fn from(number: $ty) -> Self {
                    ConstValue::from(number as u64)
                }
            }
        )*
    }
}

macro_rules! unsigned_value_try_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = $crate::runtime::RuntimeError;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, $crate::runtime::RuntimeError> {
                    match <u64>::try_from(value) {
                        Ok(number) => Ok(Value::from(number)),
                        #[allow(unreachable_patterns)]
                        Err(..) => Err($crate::runtime::RuntimeError::from(VmErrorKind::IntegerToValueCoercionError {
                            from: VmIntegerRepr::from(value),
                            to: any::type_name::<u64>(),
                        })),
                    }
                }
            }

            impl TryFrom<$ty> for ConstValue {
                type Error = $crate::runtime::RuntimeError;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, $crate::runtime::RuntimeError> {
                    match <u64>::try_from(value) {
                        Ok(number) => Ok($crate::runtime::ConstValue::from(number)),
                        #[allow(unreachable_patterns)]
                        Err(..) => Err($crate::runtime::RuntimeError::from(VmErrorKind::IntegerToValueCoercionError {
                            from: VmIntegerRepr::from(value),
                            to: any::type_name::<u64>(),
                        })),
                    }
                }
            }
        )*
    }
}

macro_rules! float_value_trait {
    ($($ty:ty),* $(,)?) => {
        $(
            impl $crate::runtime::ToValue for $ty {
                #[inline]
                fn to_value(self) -> Result<Value, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::Value::from(self as f64))
                }
            }

            impl From<$ty> for $crate::runtime::Value {
                #[inline]
                fn from(value: $ty) -> Value {
                    $crate::runtime::Value::from(value as f64)
                }
            }

            impl $crate::runtime::ToConstValue for $ty {
                #[inline]
                fn to_const_value(self) -> Result<$crate::runtime::ConstValue, $crate::runtime::RuntimeError> {
                    Ok($crate::runtime::ConstValue::from(self as f64))
                }
            }

            impl From<$ty> for $crate::runtime::ConstValue {
                #[inline]
                fn from(value: $ty) -> $crate::runtime::ConstValue {
                    $crate::runtime::ConstValue::from(value as f64)
                }
            }
        )*
    }
}
