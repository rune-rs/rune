#[cfg(feature = "rune-alloc")]
pub use rune_alloc as alloc;

#[cfg(feature = "rune-core")]
pub mod item {
    pub use rune_core::item::{Component, ComponentRef, IntoComponent, Item, ItemBuf};
}

pub mod support {
    pub use anyhow::Error;
    pub use anyhow::Result;
}

#[cfg(feature = "rune-core")]
pub use rune_core::item::{Item, ItemBuf};
