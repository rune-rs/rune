//! The core library is provided since it's used by both the `rune` crate and
//! `rune-macros` as a direct dependency.
//!
//! **YOU ARE NOT** supposed to depend on this directly. Doing so might cause
//! dependency errors since its API is not stable.

#![allow(clippy::module_inception)]
#![no_std]

extern crate alloc;

mod hash;
#[doc(hidden)]
pub use hash::ParametersBuilder;
pub use hash::{Hash, IntoHash, ToTypeHash};

mod item;
pub use self::item::{Component, ComponentRef, IntoComponent, Item, ItemBuf};

mod raw_str;
pub use self::raw_str::RawStr;

mod protocol;
pub use self::protocol::Protocol;

mod params;
pub use self::params::Params;

mod type_of;
pub use self::type_of::FullTypeOf;
