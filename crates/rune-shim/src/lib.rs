//! This crate is only used to build documentation for the `rune-core` crate.

#[cfg(feature = "rune-alloc")]
pub use rune_alloc as alloc;

#[cfg(feature = "rune-core")]
pub mod item {
    #[doc(inline)]
    pub use rune_core::item::{Component, ComponentRef, IntoComponent, Item, ItemBuf};
}

#[cfg(feature = "rune-core")]
pub mod hash {
    #[doc(inline)]
    pub use rune_core::hash::{ParametersBuilder, TooManyParameters};
}

pub mod support {
    pub use anyhow::Error;
    pub use anyhow::Result;
}

#[cfg(feature = "rune-core")]
pub mod runtime {
    use rune_core::hash::Hash;

    pub trait TypeHash {
        const HASH: Hash;
    }

    impl TypeHash for String {
        const HASH: Hash = Hash::new(1);
    }

    impl TypeHash for i64 {
        const HASH: Hash = Hash::new(2);
    }
}

#[cfg(feature = "rune-core")]
pub use self::runtime::TypeHash;

#[cfg(feature = "rune-core")]
pub use rune_core::item::{Item, ItemBuf};
