#[cfg(feature = "rune-alloc")]
pub use rune_alloc as alloc;

#[cfg(feature = "rune-core")]
pub mod compile {
    pub use rune_core::{Component, ComponentRef, IntoComponent, Item, ItemBuf};
}

pub mod support {
    pub use anyhow::Error;
    pub use anyhow::Result;
}
