/// Implement the value trait for an external type.
///
/// This is required to support the external type as a type argument in a
/// registered function.
///
/// This will be **deprecated** once (or if) [specialization] lands.
///
/// [specialization]: https://github.com/rust-lang/rust/issues/31844
#[macro_export]
macro_rules! impl_external {
    ($external:ty) => {
        impl $crate::ValueType for $external {
            fn value_type() -> $crate::Type {
                $crate::Type::Hash($crate::Hash::from_type_id(
                    std::any::TypeId::of::<$external>(),
                ))
            }

            fn type_info() -> $crate::TypeInfo {
                $crate::TypeInfo::Any(std::any::type_name::<$external>())
            }
        }

        impl $crate::FromValue for $external {
            fn from_value(value: $crate::Value) -> Result<Self, $crate::VmError> {
                let any = value.into_any()?;
                let any = any.take_downcast::<$external>()?;
                Ok(any)
            }
        }

        impl $crate::ToValue for $external {
            fn to_value(self) -> Result<$crate::Value, $crate::VmError> {
                let any = $crate::Any::new(self);
                let shared = $crate::Shared::new(any);
                Ok($crate::Value::Any(shared))
            }
        }

        impl<'a> $crate::UnsafeFromValue for &'a $external {
            type Output = *const $external;
            type Guard = $crate::RawOwnedRef;

            unsafe fn unsafe_from_value(
                value: $crate::Value,
            ) -> Result<(Self::Output, Self::Guard), $crate::VmError> {
                Ok(value.unsafe_into_any_ref()?)
            }

            unsafe fn to_arg(output: Self::Output) -> Self {
                &*output
            }
        }

        impl<'a> $crate::UnsafeFromValue for &'a mut $external {
            type Output = *mut $external;
            type Guard = $crate::RawOwnedMut;

            unsafe fn unsafe_from_value(
                value: $crate::Value,
            ) -> Result<(Self::Output, Self::Guard), $crate::VmError> {
                Ok(value.unsafe_into_any_mut()?)
            }

            unsafe fn to_arg(output: Self::Output) -> Self {
                &mut *output
            }
        }
    };
}

/// Build an implementation of `ValueType` basic of a static type.
macro_rules! impl_static_type {
    (impl <$($p:ident),*> $ty:ty => $static_type:expr) => {
        impl<$($p,)*> $crate::ValueType for $ty {
            fn value_type() -> $crate::Type {
                $crate::Type::StaticType($static_type)
            }

            fn type_info() -> $crate::TypeInfo {
                $crate::TypeInfo::StaticType($static_type)
            }
        }
    };

    ($ty:ty => $static_type:expr) => {
        impl $crate::ValueType for $ty {
            fn value_type() -> $crate::Type {
                $crate::Type::StaticType($static_type)
            }

            fn type_info() -> $crate::TypeInfo {
                $crate::TypeInfo::StaticType($static_type)
            }
        }
    };
}
