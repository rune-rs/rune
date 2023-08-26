//! Public packages that can be used to provide extract functionality to virtual
//! machines.
//!
//! These are usually included through
//! [`Context::with_default_modules`][crate::Context::with_default_modules].

pub mod any;
pub mod bytes;
#[cfg(feature = "capture-io")]
pub mod capture_io;
pub mod char;
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
#[cfg(feature = "std")]
pub mod io;
pub mod iter;
pub mod macros;
pub mod mem;
pub mod num;
pub mod object;
pub mod ops;
pub mod option;
pub mod result;
pub mod stream;
pub mod string;
pub mod test;
pub mod tuple;
pub mod vec;
