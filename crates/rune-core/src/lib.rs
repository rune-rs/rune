//! The core library is provided since it's used by both the `rune` crate and
//! `rune-macros` as a direct dependency.
//!
//! **YOU ARE NOT** supposed to depend on this directly. Doing so might cause
//! dependency errors since its API is not stable.

#![allow(clippy::module_inception)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod hash;
#[doc(hidden)]
pub use hash::ParametersBuilder;
pub use hash::{Hash, IntoHash, ToTypeHash};

mod item;
#[cfg(feature = "alloc")]
pub use self::item::Component;
pub use self::item::{ComponentRef, IntoComponent, Item, ItemBuf};

mod raw_str;
pub use self::raw_str::RawStr;

mod protocol;
pub use self::protocol::Protocol;

mod params;
pub use self::params::Params;

mod type_of;
pub use self::type_of::FullTypeOf;

#[cfg(feature = "std")]
#[doc(hidden)]
pub use std::error;

#[cfg(not(feature = "std"))]
pub mod error;
