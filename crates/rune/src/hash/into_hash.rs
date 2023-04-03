use crate::hash::Hash;

mod sealed {
    use crate::hash::{Hash, Params};
    use crate::runtime::Protocol;

    pub trait Sealed {}

    impl Sealed for &str {}
    impl Sealed for Hash {}
    impl Sealed for Protocol {}
    impl<T, P> Sealed for Params<T, P> {}
}

/// Trait for types which can be converted into a [Hash].
pub trait IntoHash: self::sealed::Sealed {
    /// Convert current type into a hash.
    fn into_hash(self) -> Hash;
}

impl IntoHash for Hash {
    #[inline]
    fn into_hash(self) -> Hash {
        self
    }
}

impl IntoHash for &str {
    #[inline]
    fn into_hash(self) -> Hash {
        Hash::of(self)
    }
}
