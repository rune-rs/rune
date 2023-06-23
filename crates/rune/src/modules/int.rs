//! The `std::int` module.

use core::cmp::Ordering;
use core::num::ParseIntError;

use crate as rune;
use crate::{ContextError, Module};

/// Construct the `std::int` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["int"]);

    module.ty::<ParseIntError>()?;

    module.function(["parse"], parse)?;
    module.function_meta(cmp)?;
    module.function_meta(to_float)?;

    module.associated_function("max", i64::max)?;
    module.associated_function("min", i64::min)?;
    module.associated_function("abs", i64::abs)?;
    module.associated_function("pow", i64::pow)?;

    module.associated_function("checked_add", i64::checked_add)?;
    module.associated_function("checked_sub", i64::checked_sub)?;
    module.associated_function("checked_div", i64::checked_div)?;
    module.associated_function("checked_mul", i64::checked_mul)?;
    module.associated_function("checked_rem", i64::checked_rem)?;

    module.associated_function("wrapping_add", i64::wrapping_add)?;
    module.associated_function("wrapping_sub", i64::wrapping_sub)?;
    module.associated_function("wrapping_div", i64::wrapping_div)?;
    module.associated_function("wrapping_mul", i64::wrapping_mul)?;
    module.associated_function("wrapping_rem", i64::wrapping_rem)?;

    module.associated_function("saturating_add", i64::saturating_add)?;
    module.associated_function("saturating_sub", i64::saturating_sub)?;
    module.associated_function("saturating_mul", i64::saturating_mul)?;
    module.associated_function("saturating_abs", i64::saturating_abs)?;
    module.associated_function("saturating_pow", i64::saturating_pow)?;

    Ok(module)
}

/// Parse an integer.
fn parse(s: &str) -> Result<i64, ParseIntError> {
    str::parse::<i64>(s)
}

/// This method returns an Ordering between self and other.
///
/// By convention, self.cmp(&other) returns the ordering matching the expression self <operator> other if true.
/// Examples
///
/// ```rune
/// use std::cmp::Ordering;
///
/// assert_eq!(5.cmp(10), Ordering::Less);
/// assert_eq!(10.cmp(5), Ordering::Greater);
/// assert_eq!(5.cmp(5), Ordering::Equal);
/// ```
#[rune::function(instance)]
#[inline]
fn cmp(lhs: i64, rhs: i64) -> Ordering {
    lhs.cmp(&rhs)
}

/// Convert a whole number to float.
#[rune::function(instance)]
#[inline]
fn to_float(value: i64) -> f64 {
    value as f64
}

crate::__internal_impl_any!(::std::int, ParseIntError);
