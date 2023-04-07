//! Public packages that can be used to provide extract functionality to virtual
//! machines.
//!
//! These are usually included through
//! [`Context::with_default_modules`][crate::Context::with_default_modules].

pub mod any;
pub mod bytes;
pub mod char;
pub mod cmp;
pub mod collections;
pub mod core;
pub mod float;
pub mod fmt;
pub mod future;
pub mod generator;
pub mod int;
pub mod io;
pub mod iter;
pub mod macros;
pub mod mem;
pub mod object;
pub mod ops;
pub mod option;
pub mod result;
pub mod stream;
pub mod string;
pub mod test;
pub mod vec;
