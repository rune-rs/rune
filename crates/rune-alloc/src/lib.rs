//! Raw extension utilities of std for Rune.
//!
//! Note that there is lots of unsafety in here. Use with caution.

// Quite a few parts copied from the Rust Project under the MIT license.
//
// Copyright 2014-2023 The Rust Project Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or https://opensource.org/licenses/MIT>, at your option. Files in the project
// may not be copied, modified, or distributed except according to those terms.

// alloc/hashbrown
//
// Copyright (c) 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or https://opensource.org/licenses/MIT>, at your option. Files in the project
// may not be copied, modified, or distributed except according to those terms.

#![no_std]
// TODO: get rid of this once we've evaluated what we want to have public.
#![allow(dead_code)]
#![cfg_attr(rune_nightly, feature(core_intrinsics))]
#![cfg_attr(rune_nightly, feature(dropck_eyepatch))]
#![cfg_attr(rune_nightly, feature(min_specialization))]
#![cfg_attr(rune_nightly, feature(ptr_sub_ptr))]
#![cfg_attr(rune_nightly, feature(set_ptr_value))]
#![cfg_attr(rune_nightly, feature(slice_ptr_len))]
#![cfg_attr(rune_nightly, feature(slice_range))]
#![cfg_attr(rune_nightly, feature(strict_provenance))]
#![cfg_attr(rune_nightly, feature(saturating_int_impl))]
#![cfg_attr(rune_nightly, feature(inline_const))]
#![cfg_attr(rune_nightly, feature(const_maybe_uninit_zeroed))]
// The only feature we use is `rustc_specialization_trait`.
#![cfg_attr(rune_nightly, allow(internal_features))]
#![cfg_attr(rune_nightly, feature(rustc_attrs))]
#![allow(clippy::comparison_chain)]
#![allow(clippy::manual_map)]
#![allow(clippy::type_complexity)]
#![allow(clippy::drop_non_drop)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc as rust_alloc;

// This is here for forward compatibility when we can support allocation-free
// execution.
#[cfg(not(feature = "alloc"))]
compile_error!("The `alloc` feature is currently required to build rune-alloc, but will change for parts of rune in the future.");

/// A `Result` aliased specialized towards an allocation [`Error`].
pub type Result<T, E = crate::error::Error> = core::result::Result<T, E>;

#[cfg(feature = "std")]
pub use std::path;
#[cfg(not(feature = "std"))]
pub mod path;

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub use self::no_std::abort;

#[cfg(feature = "std")]
pub use std::process::abort;

#[cfg(feature = "serde")]
mod serde;

#[macro_use]
mod public_macros;

#[macro_use]
mod macros;

pub use self::error::Error;
pub mod error;

pub mod str;

pub(crate) mod raw_vec;

pub use self::boxed::Box;
pub mod boxed;

pub use self::btree::{map as btree_map, map::BTreeMap};
pub use self::btree::{set as btree_set, set::BTreeSet};
pub(crate) mod btree;

pub use self::hashbrown::{map as hash_map, map::HashMap};
pub use self::hashbrown::{set as hash_set, set::HashSet};
pub mod hashbrown;

pub use self::vec::Vec;
pub mod vec;

pub use self::vec_deque::VecDeque;
pub mod vec_deque;

pub use self::string::String;
pub mod string;

pub mod alloc;

pub mod clone;

pub mod borrow;

pub mod iter;

pub mod fmt;

mod option;

pub(crate) mod hint;
pub(crate) mod ptr;
#[doc(hidden)]
pub mod slice;

pub mod prelude {
    //! Prelude for common traits used in combination with this crate which
    //! matches the behavior of the std prelude.
    pub use crate::borrow::TryToOwned;
    pub use crate::clone::{TryClone, TryCopy};
    pub use crate::iter::{IteratorExt, TryExtend, TryFromIterator, TryFromIteratorIn};
    pub use crate::option::OptionExt;
    pub use crate::string::TryToString;
}

pub mod limit;

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests;
