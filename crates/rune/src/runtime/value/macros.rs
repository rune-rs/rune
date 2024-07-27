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
            let value = match self.into_repr()? {
                ValueRepr::Mutable(value) => value.into_ref()?,
                ValueRepr::Inline(actual) => {
                    return Err(RuntimeError::expected_any(actual.type_info()));
                },
            };

            let result = Ref::try_map(value, |value| match value {
                Mutable::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has write access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $into_mut(self) -> Result<Mut<$ty>, RuntimeError> {
            let value = match self.into_repr()? {
                ValueRepr::Mutable(value) => value.into_mut()?,
                ValueRepr::Inline(actual) => {
                    return Err(RuntimeError::expected_any(actual.type_info()));
                },
            };

            let result = Mut::try_map(value, |value| match value {
                Mutable::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has read access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $borrow_ref(&self) -> Result<BorrowRef<'_, $ty>, RuntimeError> {
            let value = match self.value_ref()? {
                ValueRef::Inline(actual) => {
                    return Err(RuntimeError::expected::<$ty>(actual.type_info()));
                }
                ValueRef::Mutable(value) => value,
            };

            let result = BorrowRef::try_map(value.borrow_ref()?, |kind| match kind {
                Mutable::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has write access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $borrow_mut(&self) -> Result<BorrowMut<'_, $ty>, RuntimeError> {
            let value = match self.value_ref()? {
                ValueRef::Inline(actual) => {
                    return Err(RuntimeError::expected::<$ty>(actual.type_info()));
                }
                ValueRef::Mutable(value) => value,
            };

            let result = BorrowMut::try_map(value.borrow_mut()?, |kind| match kind {
                Mutable::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
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
            match self.take_value()? {
                OwnedValue::Mutable(Mutable::$kind(value)) => Ok(value),
                actual => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
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
                Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
                Repr::Inline(Inline::$kind(value)) => Ok(*value),
                Repr::Inline(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
                Repr::Mutable(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.borrow_ref()?.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This gets the value by mutable reference.
        #[inline]
        pub fn $as_mut(&mut self) -> Result<&mut $ty, RuntimeError> {
            match &mut self.repr {
                Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
                Repr::Inline(Inline::$kind(value)) => Ok(value),
                Repr::Inline(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
                Repr::Mutable(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.borrow_ref()?.type_info()))
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
            let value = match self.borrow_ref()? {
                ValueBorrowRef::Mutable(value) => value,
                actual => {
                    return Err(RuntimeError::expected::<$ty>(actual.type_info()));
                }
            };

            match &*value {
                Mutable::$kind(value) => Ok(value.clone()),
                actual => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
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

macro_rules! inline_from {
    ($($variant:ident => $ty:ty),* $(,)*) => {
        $(
            impl From<$ty> for Value {
                #[inline]
                fn from(value: $ty) -> Self {
                    Value::from(Inline::$variant(value))
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
                    VmResult::Ok(Value::from(self))
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
