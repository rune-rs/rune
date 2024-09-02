//! Public packages that can be used to provide extract functionality to virtual
//! machines.
//!
//! These are usually included through
//! [`Context::with_default_modules`][crate::Context::with_default_modules].

// Note: A fair amount of code and documentation in this and child modules is
// duplicated from the Rust project under the MIT license.
//
// https://github.com/rust-lang/rust
//
// Copyright 2014-2024 The Rust Project Developers

pub mod any;
pub mod bytes;
#[cfg(feature = "capture-io")]
pub mod capture_io;
pub mod char;
pub mod clone;
pub mod cmp;
pub mod collections;
pub mod core;
#[cfg(feature = "disable-io")]
pub mod disable_io;
pub mod f64;
pub mod fmt;
pub mod future;
pub mod generator;
pub mod hash;
pub mod i64;
pub mod io;
pub mod iter;
pub mod macros;
pub mod mem;
pub mod net;
pub mod num;
pub mod object;
pub mod ops;
pub mod option;
pub mod result;
pub mod slice;
pub mod stream;
pub mod string;
pub mod test;
pub mod tuple;
pub mod vec;
