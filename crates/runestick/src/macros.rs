/// Implement the value trait for an external type.
///
/// This is required to support the external type as a type argument in a
/// registered function.
///
/// This will be **deprecated** once (or if) [specialization] lands.
///
/// [specialization]: https://github.com/rust-lang/rust/issues/31844
#[macro_export]
macro_rules! decl_external {
    ($external:ty) => {
        $crate::decl_internal!($external);

        impl $crate::FromValue for $external {
            fn from_value(value: $crate::Value) -> Result<Self, $crate::ValueError> {
                let any = value.into_external()?;
                let any = any.take_downcast::<$external>()?;
                Ok(any)
            }
        }
    };
}

/// Implement the value trait for an internal type.
#[macro_export]
macro_rules! decl_internal {
    ($external:ty) => {
        impl $crate::ReflectValueType for $external {
            type Owned = $external;

            fn value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }

            fn value_type_info() -> $crate::ValueTypeInfo {
                $crate::ValueTypeInfo::External(std::any::type_name::<$external>())
            }
        }

        impl<'a> $crate::ReflectValueType for &'a $external {
            type Owned = $external;

            fn value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }

            fn value_type_info() -> $crate::ValueTypeInfo {
                $crate::ValueTypeInfo::External(std::any::type_name::<$external>())
            }
        }

        impl<'a> $crate::ReflectValueType for &'a mut $external {
            type Owned = $external;

            fn value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }

            fn value_type_info() -> $crate::ValueTypeInfo {
                $crate::ValueTypeInfo::External(std::any::type_name::<$external>())
            }
        }

        impl $crate::ToValue for $external {
            fn to_value(self) -> Result<$crate::Value, $crate::ValueError> {
                let any = $crate::Any::new(self);
                let shared = $crate::Shared::new(any);
                Ok($crate::Value::External(shared))
            }
        }

        impl<'a> $crate::UnsafeToValue for &'a $external {
            unsafe fn unsafe_to_value(self) -> Result<$crate::Value, $crate::ValueError> {
                Ok($crate::Value::from_ptr(self))
            }
        }

        impl<'a> $crate::UnsafeToValue for &'a mut $external {
            unsafe fn unsafe_to_value(self) -> Result<$crate::Value, $crate::ValueError> {
                Ok($crate::Value::from_mut_ptr(self))
            }
        }

        impl<'a> $crate::UnsafeFromValue for &'a $external {
            type Output = *const $external;
            type Guard = $crate::RawOwnedRef;

            unsafe fn unsafe_from_value(
                value: $crate::Value,
            ) -> Result<(Self::Output, Self::Guard), $crate::ValueError> {
                Ok(value.unsafe_into_external_ref()?)
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
            ) -> Result<(Self::Output, Self::Guard), $crate::ValueError> {
                Ok(value.unsafe_into_external_mut()?)
            }

            unsafe fn to_arg(output: Self::Output) -> Self {
                &mut *output
            }
        }
    };
}
