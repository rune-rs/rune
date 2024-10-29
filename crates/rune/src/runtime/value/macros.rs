/// Macro used to generate coersions for [`Value`].
macro_rules! into_base {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
    ) => {
        $(#[$($meta)*])*
        ///
        /// This ensures that the value has read access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $into_ref(self) -> Result<Ref<$ty>, RuntimeError> {
            match self.repr {
                Repr::Empty => {
                    Err(RuntimeError::from(AccessError::empty()))
                }
                Repr::Inline(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                },
                Repr::Mutable(value) => {
                    let value = value.into_ref()?;

                    let result = Ref::try_map(value, |value| match value {
                        Mutable::$kind(bytes) => Some(bytes),
                        _ => None,
                    });

                    match result {
                        Ok(bytes) => Ok(bytes),
                        Err(value) => {
                            Err(RuntimeError::expected::<$ty>(value.type_info()))
                        }
                    }
                },
                Repr::Any(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                },
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has write access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $into_mut(self) -> Result<Mut<$ty>, RuntimeError> {
            match self.repr {
                Repr::Empty => {
                    Err(RuntimeError::from(AccessError::empty()))
                }
                Repr::Inline(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                },
                Repr::Mutable(value) => {
                    let value = value.into_mut()?;

                    let result = Mut::try_map(value, |value| match value {
                        Mutable::$kind(value) => Some(value),
                        _ => None,
                    });

                    match result {
                        Ok(value) => Ok(value),
                        Err(value) => Err(RuntimeError::expected::<$ty>(value.type_info())),
                    }
                }
                Repr::Any(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                },
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has read access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $borrow_ref(&self) -> Result<BorrowRef<'_, $ty>, RuntimeError> {
            match self.as_ref_repr()? {
                RefRepr::Inline(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                },
                RefRepr::Mutable(value) => {
                    let result = BorrowRef::try_map(value.borrow_ref()?, |kind| match kind {
                        Mutable::$kind(value) => Some(value),
                        _ => None,
                    });

                    match result {
                        Ok(value) => Ok(value),
                        Err(value) => Err(RuntimeError::expected::<$ty>(value.type_info())),
                    }
                },
                RefRepr::Any(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                },
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has write access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $borrow_mut(&self) -> Result<BorrowMut<'_, $ty>, RuntimeError> {
            match self.as_ref_repr()? {
                RefRepr::Inline(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
                RefRepr::Mutable(value) => {
                    let result = BorrowMut::try_map(value.borrow_mut()?, |kind| match kind {
                        Mutable::$kind(value) => Some(value),
                        _ => None,
                    });

                    match result {
                        Ok(value) => Ok(value),
                        Err(value) => Err(RuntimeError::expected::<$ty>(value.type_info())),
                    }
                },
                RefRepr::Any(value) => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
            }
        }
    }
}

macro_rules! into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
        $into:ident,
    ) => {
        into_base! {
            $(#[$($meta)*])*
            $kind($ty),
            $into_ref,
            $into_mut,
            $borrow_ref,
            $borrow_mut,
        }

        $(#[$($meta)*])*
        ///
        /// This consumes the underlying value.
        #[inline]
        pub fn $into(self) -> Result<$ty, RuntimeError> {
            match self.take_repr()? {
                OwnedRepr::Mutable(Mutable::$kind(value)) => Ok(value),
                value => Err(RuntimeError::expected::<$ty>(value.type_info())),
            }
        }
    }
}

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
                Repr::Mutable(value) => {
                    Err(RuntimeError::expected::<$ty>(value.borrow_ref()?.type_info()))
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
                Repr::Mutable(value) => {
                    Err(RuntimeError::expected::<$ty>(value.borrow_ref()?.type_info()))
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

macro_rules! clone_into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
        $as:ident,
    ) => {
        into_base! {
            $(#[$($meta)*])*
            $kind($ty),
            $into_ref,
            $into_mut,
            $borrow_ref,
            $borrow_mut,
        }

        $(#[$($meta)*])*
        ///
        /// This clones the underlying value.
        #[inline]
        pub fn $as(&self) -> Result<$ty, RuntimeError> {
            let value = match self.borrow_ref_repr()? {
                BorrowRefRepr::Mutable(value) => value,
                value => {
                    return Err(RuntimeError::expected::<$ty>(value.type_info()));
                }
            };

            match &*value {
                Mutable::$kind(value) => Ok(value.clone()),
                value => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
            }
        }
    }
}

macro_rules! from {
    ($($variant:ident => $ty:ty),* $(,)*) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = alloc::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    Value::try_from(Mutable::$variant(value))
                }
            }

            impl IntoOutput for $ty {
                type Output = $ty;

                #[inline]
                fn into_output(self) -> VmResult<Self::Output> {
                    VmResult::Ok(self)
                }
            }

            impl ToValue for $ty {
                #[inline]
                fn to_value(self) -> VmResult<Value> {
                    VmResult::Ok(vm_try!(Value::try_from(self)))
                }
            }
        )*
    };
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
                fn to_value(self) -> $crate::runtime::VmResult<Value> {
                    $crate::runtime::VmResult::Ok($crate::runtime::Value::from(self))
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

macro_rules! from_container {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = alloc::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, alloc::Error> {
                    Value::try_from(Mutable::$variant(value))
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

macro_rules! number_value_trait {
    ($($ty:ty),* $(,)?) => {
        $(
            impl $crate::runtime::ToValue for $ty {
                #[inline]
                fn to_value(self) -> $crate::runtime::VmResult<Value> {
                    $crate::runtime::VmResult::Ok(vm_try!(Value::try_from(self)))
                }
            }

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

            impl $crate::runtime::ToConstValue for $ty {
                #[inline]
                fn to_const_value(self) -> Result<$crate::runtime::ConstValue, $crate::runtime::RuntimeError> {
                    $crate::runtime::ConstValue::try_from(self)
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
    };
}

macro_rules! float_value_trait {
    ($($ty:ty),* $(,)?) => {
        $(
            impl $crate::runtime::ToValue for $ty {
                #[inline]
                fn to_value(self) -> $crate::runtime::VmResult<$crate::runtime::Value> {
                    $crate::runtime::VmResult::Ok($crate::runtime::Value::from(self as f64))
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
