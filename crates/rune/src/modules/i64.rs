//! The `std::i64` module.

use core::cmp::Ordering;
use core::num::ParseIntError;

use crate as rune;
use crate::alloc;
use crate::alloc::string::TryToString;
use crate::runtime::{VmErrorKind, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::i64` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["i64"])?;

    module.function("parse", parse).build()?;
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

    module.function_meta(signum)?;
    module.function_meta(is_positive)?;
    module.function_meta(is_negative)?;

    module.function_meta(partial_eq)?;
    module.function_meta(eq)?;
    module.function_meta(partial_cmp)?;
    module.function_meta(cmp)?;
    module.function_meta(to_string)?;

    module.constant("MIN", i64::MIN).build()?.docs([
        "The smallest value that can be represented by this integer type",
        "(&minus;2<sup>63</sup>).",
        "",
        "# Examples",
        "",
        "Basic usage:",
        "",
        "```rune",
        "assert_eq!(i64::MIN, -9223372036854775808);",
        "```",
    ])?;

    module.constant("MAX", i64::MAX).build()?.docs([
        "The largest value that can be represented by this integer type",
        "(2<sup>63</sup> &minus; 1).",
        "",
        "# Examples",
        "",
        "Basic usage:",
        "",
        "```rune",
        "assert_eq!(i64::MAX, 9223372036854775807);",
        "```",
    ])?;

    Ok(module)
}

/// Parse an `int`.
///
/// # Examples
///
/// ```rune
/// assert_eq!(i64::parse("10")?, 10);
/// ```
fn parse(s: &str) -> Result<i64, ParseIntError> {
    str::parse::<i64>(s)
}

/// Convert an `int` to a `float`.
///
/// # Examples
///
/// ```rune
/// assert!(10.to::<f64>() is f64);
/// ```
#[rune::function(instance, path = to::<f64>)]
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
/// The absolute value of `i64::MIN` cannot be represented as an `int`, and
/// attempting to calculate it will cause an overflow. This means that such code
/// will wrap to `i64::MIN` without a panic.
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
/// assert_eq!((i64::MAX - 2).checked_add(1), Some(i64::MAX - 1));
/// assert_eq!((i64::MAX - 2).checked_add(3), None);
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
/// assert_eq!((i64::MIN + 2).checked_sub(1), Some(i64::MIN + 1));
/// assert_eq!((i64::MIN + 2).checked_sub(3), None);
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
/// assert_eq!((i64::MIN + 1).checked_div(-1), Some(i64::MAX));
/// assert_eq!(i64::MIN.checked_div(-1), None);
/// assert_eq!((1).checked_div(0), None);
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
/// assert_eq!(i64::MAX.checked_mul(1), Some(i64::MAX));
/// assert_eq!(i64::MAX.checked_mul(2), None);
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
/// assert_eq!(i64::MIN.checked_rem(-1), None);
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
/// assert_eq!(i64::MAX.wrapping_add(2), i64::MIN + 1);
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
// assert_eq!((-2).wrapping_sub(i64::MAX), i64::MAX);
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
/// assert_eq!(i64::MAX.saturating_add(100), i64::MAX);
/// assert_eq!(i64::MIN.saturating_add(-1), i64::MIN);
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
/// assert_eq!(i64::MIN.saturating_sub(100), i64::MIN);
/// assert_eq!(i64::MAX.saturating_sub(-1), i64::MAX);
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
/// assert_eq!(i64::MAX.saturating_mul(10), i64::MAX);
/// assert_eq!(i64::MIN.saturating_mul(10), i64::MIN);
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
/// assert_eq!(i64::MIN.saturating_abs(), i64::MAX);
/// assert_eq!((i64::MIN + 1).saturating_abs(), i64::MAX);
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
/// assert_eq!(i64::MIN.saturating_pow(2), i64::MAX);
/// assert_eq!(i64::MIN.saturating_pow(3), i64::MIN);
/// ```
#[rune::function(instance)]
#[inline]
fn saturating_pow(this: i64, rhs: u32) -> i64 {
    i64::saturating_pow(this, rhs)
}

/// Returns a number representing sign of `self`.
///
/// - `0` if the number is zero
/// - `1` if the number is positive
/// - `-1` if the number is negative
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!(10.signum(), 1);
/// assert_eq!(0.signum(), 0);
/// assert_eq!((-10).signum(), -1);
/// ```
#[rune::function(instance)]
#[inline]
fn signum(this: i64) -> i64 {
    i64::signum(this)
}

/// Returns `true` if `self` is positive and `false` if the number is zero or
/// negative.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!(10.is_positive());
/// assert!(!(-10).is_positive());
/// ```
#[rune::function(instance)]
#[inline]
fn is_positive(this: i64) -> bool {
    i64::is_positive(this)
}

/// Returns `true` if `self` is negative and `false` if the number is zero or
/// positive.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!((-10).is_negative());
/// assert!(!10.is_negative());
/// ```
#[rune::function(instance)]
#[inline]
fn is_negative(this: i64) -> bool {
    i64::is_negative(this)
}

/// Test two integers for partial equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::partial_eq;
///
/// assert_eq!(partial_eq(5, 5), true);
/// assert_eq!(partial_eq(5, 10), false);
/// assert_eq!(partial_eq(10, 5), false);
/// ```
#[rune::function(instance, protocol = PARTIAL_EQ)]
#[inline]
fn partial_eq(this: i64, rhs: i64) -> bool {
    this.eq(&rhs)
}

/// Test two integers for total equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// assert_eq!(eq(5, 5), true);
/// assert_eq!(eq(5, 10), false);
/// assert_eq!(eq(10, 5), false);
/// ```
#[rune::function(instance, protocol = EQ)]
#[inline]
fn eq(this: i64, rhs: i64) -> bool {
    this.eq(&rhs)
}

/// Perform a partial ordered comparison between two integers.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::partial_cmp;
///
/// assert_eq!(partial_cmp(5, 10), Some(Ordering::Less));
/// assert_eq!(partial_cmp(10, 5), Some(Ordering::Greater));
/// assert_eq!(partial_cmp(5, 5), Some(Ordering::Equal));
/// ```
#[rune::function(instance, protocol = PARTIAL_CMP)]
#[inline]
fn partial_cmp(this: i64, rhs: i64) -> Option<Ordering> {
    this.partial_cmp(&rhs)
}

/// Perform a partial ordered comparison between two integers.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::cmp;
///
/// assert_eq!(cmp(5, 10), Ordering::Less);
/// assert_eq!(cmp(10, 5), Ordering::Greater);
/// assert_eq!(cmp(5, 5), Ordering::Equal);
/// ```
#[rune::function(instance, protocol = CMP)]
#[inline]
fn cmp(this: i64, rhs: i64) -> Ordering {
    this.cmp(&rhs)
}

/// Returns the number as a string.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!((-10).to_string(), "-10");
/// assert_eq!(10.to_string(), "10");
/// ```
#[rune::function(instance)]
#[inline]
fn to_string(this: i64) -> VmResult<alloc::String> {
    VmResult::Ok(vm_try!(this.try_to_string()))
}
