//! Integers.

use core::cmp::Ordering;
use core::num::ParseIntError;

use crate as rune;
use crate::alloc;
use crate::alloc::string::TryToString;
use crate::{ContextError, Module};

/// Signed integers.
///
/// This provides methods for computing over and parsing 64-bit signed integers.
#[rune::module(::std::i64)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;
    signed!(m, i64);
    Ok(m)
}

signed_fns!(i64);
