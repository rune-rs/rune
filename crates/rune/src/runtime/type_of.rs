use crate::runtime::{Mut, Ref, Shared, TypeInfo};
use crate::Hash;

#[doc(inline)]
pub use rune_core::FullTypeOf;

/// Trait used for Rust types for which we can determine the runtime type of.
pub trait TypeOf {
    /// Type information for the given type.
    #[inline]
    fn type_of() -> FullTypeOf {
        FullTypeOf::new(Self::type_hash())
    }

    /// Hash of type parameters.
    #[inline]
    fn type_parameters() -> Hash {
        Hash::EMPTY
    }

    /// Get full type hash, including type parameters.
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
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

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
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

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
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

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
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

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
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}
