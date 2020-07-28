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
            fn reflect_value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }
        }

        impl<'a> $crate::ReflectValueType for &'a $external {
            fn reflect_value_type() -> $crate::ValueType {
                $crate::ValueType::External(std::any::TypeId::of::<$external>())
            }
        }

        impl $crate::ToValue for $external {
            fn to_value(self, vm: &mut $crate::Vm) -> Option<$crate::ValueRef> {
                Some(vm.allocate_external(self))
            }
        }

        impl $crate::FromValue for $external {
            fn from_value(
                value: $crate::ValueRef,
                vm: &$crate::Vm,
            ) -> Result<Self, $crate::ValueRef> {
                let slot = value.into_external()?;

                match vm.external_clone::<$external>(slot) {
                    Some(value) => Ok(value),
                    None => Err(value),
                }
            }
        }

        impl<'a> $crate::UnsafeFromValue for &'a $external {
            unsafe fn unsafe_from_value(
                value: $crate::ValueRef,
                vm: &$crate::Vm,
            ) -> Result<Self, $crate::ValueRef> {
                let slot = value.into_external()?;

                match vm.external_ref::<$external>(slot) {
                    Some(value) => Ok(std::mem::transmute(value)),
                    None => Err(value),
                }
            }
        }
    };
}
