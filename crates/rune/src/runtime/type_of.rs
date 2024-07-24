use crate::alloc;
use crate::compile::meta;
use crate::runtime::{Mut, Ref, Shared, TypeInfo};
use crate::Hash;

/// Core type of trait.
pub trait CoreTypeOf {
    /// Get full type hash, including type parameters.
    fn type_hash() -> Hash;
}

/// Blanket implementation for references.
impl<T> CoreTypeOf for &T
where
    T: ?Sized + CoreTypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }
}

/// Blanket implementation for mutable references.
impl<T> CoreTypeOf for &mut T
where
    T: ?Sized + CoreTypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }
}

/// Trait used for Rust types for which we can determine the runtime type of.
pub trait TypeOf: CoreTypeOf {
    /// Hash of type parameters.
    #[inline]
    fn type_parameters() -> Hash {
        Hash::EMPTY
    }

    /// Access diagnostical information on the value type.
    fn type_info() -> TypeInfo;
}

/// A type that might or might not have a concrete type.
pub trait MaybeTypeOf {
    /// Type information for the given type.
    fn maybe_type_of() -> alloc::Result<meta::DocType>;
}

impl<T> MaybeTypeOf for &T
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for &mut T
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for Ref<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for Mut<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for Shared<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        T::maybe_type_of()
    }
}

/// Blanket implementation for references.
impl<T> TypeOf for &T
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for mutable references.
impl<T> TypeOf for &mut T
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned references.
impl<T> CoreTypeOf for Ref<T>
where
    T: ?Sized + CoreTypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }
}

/// Blanket implementation for owned references.
impl<T> TypeOf for Ref<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned mutable references.
impl<T> CoreTypeOf for Mut<T>
where
    T: ?Sized + CoreTypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }
}

/// Blanket implementation for owned mutable references.
impl<T> TypeOf for Mut<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned shared values.
impl<T> CoreTypeOf for Shared<T>
where
    T: ?Sized + CoreTypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }
}

/// Blanket implementation for owned shared values.
impl<T> TypeOf for Shared<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}
