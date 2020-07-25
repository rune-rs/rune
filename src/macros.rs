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
                Ok(state.allocate($crate::Value::External(index)))
            }
        }

        impl $crate::ReflectToValue for $external {
            fn reflect_to_value(self, state: &mut $crate::State) -> Option<$crate::Value> {
                Some($crate::Value::External(state.allocate_external(self)))
            }
        }

        impl $crate::ReflectFromValue for $external {
            fn reflect_from_value(
                value: $crate::Value,
                state: &$crate::State,
            ) -> Result<Self, $crate::Value> {
                match value {
                    $crate::Value::External(index) => match state.cloned_external::<Self>(index) {
                        Some(value) => Ok(value),
                        None => return Err($crate::Value::External(index)),
                    },
                    value => Err(value),
                }
            }
        }
    };
}
