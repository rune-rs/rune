use crate::{Type, TypeInfo};

/// Trait used for Rust types for which we can determine the runtime type of.
pub trait TypeOf {
    /// Convert into a value type.
    fn type_of() -> Type;

    /// Access diagnostical information on the value type.
    fn type_info() -> TypeInfo;
}

/// Blanket implementation for references.
impl<T: ?Sized> TypeOf for &T
where
    T: TypeOf,
{
    fn type_of() -> Type {
        T::type_of()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for mutable references.
impl<T: ?Sized> TypeOf for &mut T
where
    T: TypeOf,
{
    fn type_of() -> Type {
        T::type_of()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}
