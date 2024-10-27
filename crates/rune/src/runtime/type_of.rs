use crate::alloc;
use crate::compile::meta;
use crate::runtime::{Mut, Ref, Shared, TypeInfo};
use crate::Hash;

/// Static type hash for a given type.
///
/// This trait allows you to determine the unique hash of any given type that
/// can be used in Rune through the [`HASH`] associated constant.
///
/// This trait is usually implemented automatically through the [`Any` derive].
///
/// A type hash is unique for types which in Rune are considered the same. This
/// might not be true for types in Rust. For example, `&str` and `String` have
/// the same type hash:
///
/// ```
/// use rune::TypeHash;
///
/// assert_eq!(<&str>::HASH, String::HASH);
/// ```
///
/// [`HASH`]: TypeHash::HASH
/// [`Any` derive]: derive@crate::Any
pub trait TypeHash {
    /// The complete type hash of the type including type parameters which
    /// uniquely identifiers a given type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::TypeHash;
    ///
    /// assert_ne!(String::HASH, i64::HASH);
    ///
    /// fn is_a_string<T>() -> bool where T: TypeHash {
    ///     matches!(T::HASH, String::HASH)
    /// }
    ///
    /// assert!(is_a_string::<String>());
    /// assert!(!is_a_string::<i64>());
    /// ```
    const HASH: Hash;
}

/// Blanket implementation for references.
impl<T> TypeHash for &T
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

/// Blanket implementation for mutable references.
impl<T> TypeHash for &mut T
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

/// Trait used for Rust types for which we can determine the runtime type of.
pub trait TypeOf: TypeHash {
    /// Type parameters for the type.
    ///
    /// See [`ParametersBuilder`] for more information.
    ///
    /// [`ParametersBuilder`]: crate::hash::ParametersBuilder
    const PARAMETERS: Hash = Hash::EMPTY;

    /// Diagnostical type information for the current type.
    ///
    /// Has reasonable [`Debug`] and [`Display`] implementations to identify a
    /// given type.
    ///
    /// [`Debug`]: core::fmt::Debug
    /// [`Display`]: core::fmt::Display
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
    const PARAMETERS: Hash = T::PARAMETERS;

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
    const PARAMETERS: Hash = T::PARAMETERS;

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned references.
impl<T> TypeHash for Ref<T>
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

/// Blanket implementation for owned references.
impl<T> TypeOf for Ref<T>
where
    T: ?Sized + TypeOf,
{
    const PARAMETERS: Hash = T::PARAMETERS;

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned mutable references.
impl<T> TypeHash for Mut<T>
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

/// Blanket implementation for owned mutable references.
impl<T> TypeOf for Mut<T>
where
    T: ?Sized + TypeOf,
{
    const PARAMETERS: Hash = T::PARAMETERS;

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Blanket implementation for owned shared values.
impl<T> TypeHash for Shared<T>
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

/// Blanket implementation for owned shared values.
impl<T> TypeOf for Shared<T>
where
    T: ?Sized + TypeOf,
{
    const PARAMETERS: Hash = T::PARAMETERS;

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}
