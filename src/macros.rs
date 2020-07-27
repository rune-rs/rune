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
        impl $crate::ReflectValueType for $external {
            fn reflect_value_type() -> $crate::ValueType {
                $crate::ValueType::External($crate::TypeHash::of::<Self>())
            }
        }

        impl $crate::Allocate for $external {
            fn allocate(self, state: &mut $crate::State) -> Result<usize, $crate::AllocateError> {
                let index = state.allocate_external(self);
                Ok(state.allocate($crate::ValueRef::External(index)))
            }
        }

        impl $crate::ToValue for $external {
            fn to_value(self, state: &mut $crate::State) -> Option<$crate::ValueRef> {
                Some($crate::ValueRef::External(state.allocate_external(self)))
            }
        }

        impl $crate::FromValue for $external {
            fn from_value(
                value: $crate::ValueRef,
                state: &$crate::State,
            ) -> Result<Self, $crate::ValueRef> {
                match value {
                    $crate::ValueRef::External(index) => match state.cloned_external::<Self>(index)
                    {
                        Some(value) => Ok(value),
                        None => return Err($crate::ValueRef::External(index)),
                    },
                    value => Err(value),
                }
            }
        }
    };
}
