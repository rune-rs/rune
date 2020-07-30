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
        impl $crate::ReflectValueType for $external {
            fn value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }

            fn value_type_info() -> $crate::ValueTypeInfo {
                $crate::ValueTypeInfo::External(std::any::type_name::<$external>())
            }
        }

        impl<'a> $crate::ReflectValueType for &'a $external {
            fn value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }

            fn value_type_info() -> $crate::ValueTypeInfo {
                $crate::ValueTypeInfo::External(std::any::type_name::<$external>())
            }
        }

        impl<'a> $crate::ReflectValueType for &'a mut $external {
            fn value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }

            fn value_type_info() -> $crate::ValueTypeInfo {
                $crate::ValueTypeInfo::External(std::any::type_name::<$external>())
            }
        }

        impl $crate::ToValue for $external {
            fn to_value(self, vm: &mut $crate::Vm) -> Result<$crate::ValuePtr, $crate::StackError> {
                Ok(vm.external_allocate(self))
            }
        }

        impl $crate::FromValue for $external {
            fn from_value(
                value: $crate::ValuePtr,
                vm: &mut $crate::Vm,
            ) -> Result<Self, $crate::StackError> {
                let slot = value.into_external()?;
                vm.external_take::<$external>(slot)
            }
        }

        impl<'a> $crate::UnsafeFromValue for &'a $external {
            unsafe fn unsafe_from_value(
                value: $crate::ValuePtr,
                vm: &mut $crate::Vm,
            ) -> Result<Self, $crate::StackError> {
                let slot = value.into_external()?;
                Ok($crate::Ref::unsafe_into_ref(
                    vm.external_ref::<$external>(slot)?,
                ))
            }
        }

        impl<'a> $crate::UnsafeFromValue for &'a mut $external {
            unsafe fn unsafe_from_value(
                value: $crate::ValuePtr,
                vm: &mut $crate::Vm,
            ) -> Result<Self, $crate::StackError> {
                let slot = value.into_external()?;
                Ok($crate::Mut::unsafe_into_mut(
                    vm.external_mut::<$external>(slot)?,
                ))
            }
        }
    };
}
