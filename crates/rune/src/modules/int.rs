//! The `std::int` module.

use core::cmp::Ordering;
use core::num::ParseIntError;

use crate as rune;
use crate::runtime::{VmErrorKind, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::int` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["int"]);

    module.ty::<ParseIntError>()?;

    module.function(["parse"], parse)?;
    module.function_meta(cmp)?;
    module.function_meta(to_float)?;

    module.function_meta(max)?;
    module.function_meta(min)?;
    module.function_meta(abs)?;
    module.function_meta(pow)?;

    module.function_meta(checked_add)?;
    module.function_meta(checked_sub)?;
    module.function_meta(checked_div)?;
    module.function_meta(checked_mul)?;
    module.function_meta(checked_rem)?;

    module.function_meta(wrapping_add)?;
    module.function_meta(wrapping_sub)?;
    module.function_meta(wrapping_div)?;
    module.function_meta(wrapping_mul)?;
    module.function_meta(wrapping_rem)?;

    module.function_meta(saturating_add)?;
    module.function_meta(saturating_sub)?;
    module.function_meta(saturating_mul)?;
    module.function_meta(saturating_abs)?;
    module.function_meta(saturating_pow)?;

    module.constant(["MIN"], i64::MIN)?.docs([
        "The smallest value that can be represented by this integer type",
        "(&minus;2<sup>63</sup>).",
        "",
        "# Examples",
        "",
        "Basic usage:",
        "",
        "```rune",
        "assert_eq!(int::MIN, -9223372036854775808);",
        "```",
    ]);

    module.constant(["MAX"], i64::MAX)?.docs([
        "The largest value that can be represented by this integer type",
        "(2<sup>63</sup> &minus; 1).",
        "",
        "# Examples",
        "",
        "Basic usage:",
        "",
        "```rune",
        "assert_eq!(int::MAX, 9223372036854775807);",
        "```",
    ]);

    Ok(module)
}

/// Parse an `int`.
///
/// # Examples
///
/// ```rune
/// assert_eq!(int::parse("10")?, 10);
/// ```
fn parse(s: &str) -> Result<i64, ParseIntError> {
    str::parse::<i64>(s)
}

/// This method returns an Ordering between self and other.
///
/// By convention, `self.cmp(other)` returns the ordering matching the
/// expression `self <operator> other` if true.
///
/// # Examples
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
fn cmp(this: i64, rhs: i64) -> Ordering {
    this.cmp(&rhs)
}

/// Convert an `int` to a `float`.
///
/// # Examples
///
/// ```rune
/// assert!(10.to_float() is float);
/// ```
#[rune::function(instance)]
#[inline]
fn to_float(value: i64) -> f64 {
    value as f64
}

/// Compares and returns the maximum of two values.
///
/// Returns the second argument if the comparison determines them to be equal.
///
/// # Examples
///
/// ```rune
/// assert_eq!(2, 1.max(2));
/// assert_eq!(2, 2.max(2));
/// ```
#[rune::function(instance)]
#[inline]
fn max(this: i64, other: i64) -> i64 {
    i64::max(this, other)
}

/// Compares and returns the minimum of two values.
///
/// Returns the first argument if the comparison determines them to be equal.
///
/// # Examples
///
/// ```rune
/// assert_eq!(1, 1.min(2));
/// assert_eq!(2, 2.min(2));
/// ```
#[rune::function(instance)]
#[inline]
fn min(this: i64, other: i64) -> i64 {
    i64::min(this, other)
}

/// Computes the absolute value of `self`.
///
/// # Overflow behavior
///
/// The absolute value of `int::MIN` cannot be represented as an `int`, and
/// attempting to calculate it will cause an overflow. This means that such code
/// will wrap to `int::MIN` without a panic.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(10.abs(), 10);
/// assert_eq!((-10).abs(), 10);
/// ```
#[rune::function(instance)]
#[inline]
fn abs(this: i64) -> i64 {
    i64::wrapping_abs(this)
}

/// Raises self to the power of `exp`, using exponentiation by squaring.
///
/// # Overflow behavior
///
/// This function will wrap on overflow.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let x = 2;
///
/// assert_eq!(x.pow(5), 32);
/// ```
#[rune::function(instance)]
#[inline]
fn pow(this: i64, pow: u32) -> i64 {
    i64::wrapping_pow(this, pow)
}

/// Checked integer addition. Computes `self + rhs`, returning `None` if
/// overflow occurred.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!((int::MAX - 2).checked_add(1), Some(int::MAX - 1));
/// assert_eq!((int::MAX - 2).checked_add(3), None);
/// ```
#[rune::function(instance)]
#[inline]
fn checked_add(this: i64, rhs: i64) -> Option<i64> {
    i64::checked_add(this, rhs)
}

/// Checked integer subtraction. Computes `self - rhs`, returning `None` if
/// overflow occurred.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!((int::MIN + 2).checked_sub(1), Some(int::MIN + 1));
/// assert_eq!((int::MIN + 2).checked_sub(3), None);
/// ```
#[rune::function(instance)]
#[inline]
fn checked_sub(this: i64, rhs: i64) -> Option<i64> {
    i64::checked_sub(this, rhs)
}

/// Checked integer division. Computes `self / rhs`, returning `None` if `rhs ==
/// 0` or the division results in overflow.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!((int::MIN + 1).checked_div(-1), Some(int::MAX));")]
/// assert_eq!(int::MIN.checked_div(-1), None);")]
/// assert_eq!((1).checked_div(0), None);")]
/// ```
#[rune::function(instance)]
#[inline]
fn checked_div(this: i64, rhs: i64) -> Option<i64> {
    i64::checked_div(this, rhs)
}

/// Checked integer multiplication. Computes `self * rhs`, returning `None` if
/// overflow occurred.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(int::MAX.checked_mul(1), Some(int::MAX));
/// assert_eq!(int::MAX.checked_mul(2), None);
/// ```
#[rune::function(instance)]
#[inline]
fn checked_mul(this: i64, rhs: i64) -> Option<i64> {
    i64::checked_mul(this, rhs)
}

/// Checked integer remainder. Computes `self % rhs`, returning `None` if `rhs
/// == 0` or the division results in overflow.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(5.checked_rem(2), Some(1));
/// assert_eq!(5.checked_rem(0), None);
/// assert_eq!(int::MIN.checked_rem(-1), None);
/// ```
#[rune::function(instance)]
#[inline]
fn checked_rem(this: i64, rhs: i64) -> Option<i64> {
    i64::checked_rem(this, rhs)
}

/// Wrapping (modular) addition. Computes `self + rhs`, wrapping around at the
/// boundary of the type.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(100.wrapping_add(27), 127);
/// assert_eq!(int::MAX.wrapping_add(2), int::MIN + 1);
/// ```
#[rune::function(instance)]
#[inline]
fn wrapping_add(this: i64, rhs: i64) -> i64 {
    i64::wrapping_add(this, rhs)
}

/// Wrapping (modular) subtraction. Computes `self - rhs`, wrapping around at
/// the boundary of the type.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
// assert_eq!(0.wrapping_sub(127), -127);
// assert_eq!((-2).wrapping_sub(int::MAX), int::MAX);
/// ```
#[rune::function(instance)]
#[inline]
fn wrapping_sub(this: i64, rhs: i64) -> i64 {
    i64::wrapping_sub(this, rhs)
}

/// Wrapping (modular) division. Computes `self / rhs`, wrapping around at the
/// boundary of the type.
///
/// The only case where such wrapping can occur is when one divides `MIN / -1`
/// on a signed type (where `MIN` is the negative minimal value for the type);
/// this is equivalent to `-MIN`, a positive value that is too large to
/// represent in the type. In such a case, this function returns `MIN` itself.
///
/// # Panics
///
/// This function will panic if `rhs` is 0.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(100.wrapping_div(10), 10);
/// ```
#[rune::function(instance)]
#[inline]
fn wrapping_div(this: i64, rhs: i64) -> VmResult<i64> {
    if rhs == 0 {
        return VmResult::err(VmErrorKind::DivideByZero);
    }

    VmResult::Ok(i64::wrapping_div(this, rhs))
}

/// Wrapping (modular) multiplication. Computes `self * rhs`, wrapping around at
/// the boundary of the type.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(10.wrapping_mul(12), 120);
/// ```
#[rune::function(instance)]
#[inline]
fn wrapping_mul(this: i64, rhs: i64) -> i64 {
    i64::wrapping_mul(this, rhs)
}

/// Wrapping (modular) remainder. Computes `self % rhs`, wrapping around at the
/// boundary of the type.
///
/// Such wrap-around never actually occurs mathematically; implementation
/// artifacts make `x % y` invalid for `MIN / -1` on a signed type (where `MIN`
/// is the negative minimal value). In such a case, this function returns `0`.
///
/// # Panics
///
/// This function will panic if `rhs` is 0.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(100.wrapping_rem(10), 0);
/// ```
#[rune::function(instance)]
#[inline]
fn wrapping_rem(this: i64, rhs: i64) -> VmResult<i64> {
    if rhs == 0 {
        return VmResult::err(VmErrorKind::DivideByZero);
    }

    VmResult::Ok(i64::wrapping_rem(this, rhs))
}

/// Saturating integer addition. Computes `self + rhs`, saturating at the
/// numeric bounds instead of overflowing.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(100.saturating_add(1), 101);
/// assert_eq!(int::MAX.saturating_add(100), int::MAX);
/// assert_eq!(int::MIN.saturating_add(-1), int::MIN);
/// ```
#[rune::function(instance)]
#[inline]
fn saturating_add(this: i64, rhs: i64) -> i64 {
    i64::saturating_add(this, rhs)
}

/// Saturating integer subtraction. Computes `self - rhs`, saturating at the
/// numeric bounds instead of overflowing.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(100.saturating_sub(127), -27);
/// assert_eq!(int::MIN.saturating_sub(100), int::MIN);
/// assert_eq!(int::MAX.saturating_sub(-1), int::MAX);
/// ```
#[rune::function(instance)]
#[inline]
fn saturating_sub(this: i64, rhs: i64) -> i64 {
    i64::saturating_sub(this, rhs)
}

/// Saturating integer multiplication. Computes `self * rhs`, saturating at the
/// numeric bounds instead of overflowing.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(10.saturating_mul(12), 120);
/// assert_eq!(int::MAX.saturating_mul(10), int::MAX);
/// assert_eq!(int::MIN.saturating_mul(10), int::MIN);
/// ```
#[rune::function(instance)]
#[inline]
fn saturating_mul(this: i64, rhs: i64) -> i64 {
    i64::saturating_mul(this, rhs)
}

/// Saturating absolute value. Computes `self.abs()`, returning `MAX` if `self
/// == MIN` instead of overflowing.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(100.saturating_abs(), 100);
/// assert_eq!((-100).saturating_abs(), 100);
/// assert_eq!(int::MIN.saturating_abs(), int::MAX);
/// assert_eq!((int::MIN + 1).saturating_abs(), int::MAX);
/// ```
#[rune::function(instance)]
#[inline]
fn saturating_abs(this: i64) -> i64 {
    i64::saturating_abs(this)
}

/// Saturating integer exponentiation. Computes `self.pow(exp)`, saturating at
/// the numeric bounds instead of overflowing.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!((-4).saturating_pow(3), -64);
/// assert_eq!(int::MIN.saturating_pow(2), int::MAX);
/// assert_eq!(int::MIN.saturating_pow(3), int::MIN);
/// ```
#[rune::function(instance)]
#[inline]
fn saturating_pow(this: i64, rhs: u32) -> i64 {
    i64::saturating_pow(this, rhs)
}

crate::__internal_impl_any!(::std::int, ParseIntError);
