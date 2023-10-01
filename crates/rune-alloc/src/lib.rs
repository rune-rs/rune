//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-alloc"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-alloc.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-alloc"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--alloc-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.70+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! The Rune Language, an embeddable dynamic programming language for Rust.
// Quite a few parts copied from the Rust Project under the MIT license.
//
// Copyright 2014-2023 The Rust Project Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or https://opensource.org/licenses/MIT>, at your option. Files in the project
// may not be copied, modified, or distributed except according to those terms.

// hashbrown
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
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_doc_tests)]
#![cfg_attr(rune_nightly, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(rune_nightly, deny(rustdoc::missing_doc_code_examples))]
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

pub mod callable;

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
