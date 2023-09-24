use crate::hash::Hash;

mod sealed {
    use crate::hash::Hash;
    use crate::params::Params;
    use crate::protocol::Protocol;

    pub trait Sealed {}

    impl Sealed for &str {}
    impl Sealed for Hash {}
    impl Sealed for Protocol {}
    impl<T, const N: usize> Sealed for Params<T, N> {}
}

/// Trait for types which can be converted into a
/// [Hash][struct@crate::hash::Hash].
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
        Hash::ident(self)
    }
}
