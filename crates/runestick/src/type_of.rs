use crate::{Hash, TypeInfo};

/// Trait used for Rust types for which we can determine the runtime type of.
pub trait TypeOf {
    /// Convert into a type hash.
    fn type_hash() -> Hash;

    /// Access diagnostical information on the value type.
    fn type_info() -> TypeInfo;
}

/// Blanket implementation for references.
impl<T: ?Sized> TypeOf for &T
where
    T: TypeOf,
{
    fn type_hash() -> Hash {
        T::type_hash()
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
    fn type_hash() -> Hash {
        T::type_hash()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned references.
impl<T: ?Sized> TypeOf for crate::Ref<T>
where
    T: TypeOf,
{
    fn type_hash() -> Hash {
        T::type_hash()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned mutable references.
impl<T: ?Sized> TypeOf for crate::Mut<T>
where
    T: TypeOf,
{
    fn type_hash() -> Hash {
        T::type_hash()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}
