use crate::runtime::{Mut, Ref, Shared, TypeInfo};
use crate::Hash;

/// Full type information.
#[derive(Debug, Clone)]
pub struct FullTypeOf {
    pub(crate) hash: Hash,
}

/// Trait used for Rust types for which we can determine the runtime type of.
pub trait TypeOf {
    /// Type information for the given type.
    #[inline]
    fn type_of() -> FullTypeOf {
        FullTypeOf {
            hash: Self::type_hash(),
        }
    }

    /// Convert into a type hash.
    fn type_hash() -> Hash;

    /// Access diagnostical information on the value type.
    fn type_info() -> TypeInfo;
}

/// A type that might or might not have a concrete type.
pub trait MaybeTypeOf {
    /// Type information for the given type.
    fn maybe_type_of() -> Option<FullTypeOf>;
}

impl<T> MaybeTypeOf for &T
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for &mut T
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for Ref<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for Mut<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<T> MaybeTypeOf for Shared<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

/// Blanket implementation for references.
impl<T> TypeOf for &T
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
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
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned references.
impl<T> TypeOf for Ref<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned mutable references.
impl<T> TypeOf for Mut<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned shared values.
impl<T> TypeOf for Shared<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}
