//! The `std::f64` module.

use core::cmp::Ordering;
use core::num::ParseFloatError;

use crate as rune;
use crate::runtime::{VmErrorKind, VmResult};
use crate::{ContextError, Module};

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["f64"])?;

    m.function_meta(parse)?
        .deprecated("Use std::string::parse::<f64> instead")?;
    m.function_meta(is_nan)?;
    m.function_meta(is_infinite)?;
    m.function_meta(is_finite)?;
    m.function_meta(is_subnormal)?;
    m.function_meta(is_normal)?;
    m.function_meta(max)?;
    m.function_meta(min)?;
    #[cfg(feature = "std")]
    m.function_meta(abs)?;
    #[cfg(feature = "std")]
    m.function_meta(powf)?;
    #[cfg(feature = "std")]
    m.function_meta(powi)?;
    m.function_meta(to_integer)?;
    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;

    m.constant("EPSILON", f64::EPSILON).build()?;
    m.constant("MIN", f64::MIN).build()?;
    m.constant("MAX", f64::MAX).build()?;
    m.constant("MIN_POSITIVE", f64::MIN_POSITIVE).build()?;
    m.constant("MIN_EXP", f64::MIN_EXP).build()?;
    m.constant("MAX_EXP", f64::MAX_EXP).build()?;
    m.constant("MIN_10_EXP", f64::MIN_10_EXP).build()?;
    m.constant("MAX_10_EXP", f64::MAX_10_EXP).build()?;
    m.constant("NAN", f64::NAN).build()?;
    m.constant("INFINITY", f64::INFINITY).build()?;
    m.constant("NEG_INFINITY", f64::NEG_INFINITY).build()?;
    Ok(m)
}

#[rune::function]
fn parse(s: &str) -> Result<f64, ParseFloatError> {
    str::parse::<f64>(s)
}

/// Convert a float to a an integer.
///
/// # Examples
///
/// ```rune
/// let n = 7.0_f64.to::<i64>();
/// assert_eq!(n, 7);
/// ```
#[rune::function(instance, path = to::<i64>)]
fn to_integer(value: f64) -> i64 {
    value as i64
}

/// Returns `true` if this value is NaN.
///
/// # Examples
///
/// ```rune
/// let nan = f64::NAN;
/// let f = 7.0_f64;
///
/// assert!(nan.is_nan());
/// assert!(!f.is_nan());
/// ```
#[rune::function(instance)]
fn is_nan(this: f64) -> bool {
    this.is_nan()
}

/// Returns `true` if this value is positive infinity or negative infinity, and
/// `false` otherwise.
///
/// # Examples
///
/// ```rune
/// let f = 7.0f64;
/// let inf = f64::INFINITY;
/// let neg_inf = f64::NEG_INFINITY;
/// let nan = f64::NAN;
///
/// assert!(!f.is_infinite());
/// assert!(!nan.is_infinite());
///
/// assert!(inf.is_infinite());
/// assert!(neg_inf.is_infinite());
/// ```
#[rune::function(instance)]
fn is_infinite(this: f64) -> bool {
    this.is_infinite()
}

/// Returns `true` if this number is neither infinite nor NaN.
///
/// # Examples
///
/// ```rune
/// let f = 7.0f64;
/// let inf = f64::INFINITY;
/// let neg_inf = f64::NEG_INFINITY;
/// let nan = f64::NAN;
///
/// assert!(f.is_finite());
///
/// assert!(!nan.is_finite());
/// assert!(!inf.is_finite());
/// assert!(!neg_inf.is_finite());
/// ```
#[rune::function(instance)]
fn is_finite(this: f64) -> bool {
    this.is_finite()
}

/// Returns `true` if the number is [subnormal].
///
/// # Examples
///
/// ```rune
/// let min = f64::MIN_POSITIVE; // 2.2250738585072014e-308_f64
/// let max = f64::MAX;
/// let lower_than_min = 1.0e-308_f64;
/// let zero = 0.0_f64;
///
/// assert!(!min.is_subnormal());
/// assert!(!max.is_subnormal());
///
/// assert!(!zero.is_subnormal());
/// assert!(!f64::NAN.is_subnormal());
/// assert!(!f64::INFINITY.is_subnormal());
/// // Values between `0` and `min` are Subnormal.
/// assert!(lower_than_min.is_subnormal());
/// ```
///
/// [subnormal]: https://en.wikipedia.org/wiki/Denormal_number
#[rune::function(instance)]
fn is_subnormal(this: f64) -> bool {
    this.is_subnormal()
}

/// Returns `true` if the number is neither zero, infinite, [subnormal], or NaN.
///
/// # Examples
///
/// ```rune
/// let min = f64::MIN_POSITIVE; // 2.2250738585072014e-308f64
/// let max = f64::MAX;
/// let lower_than_min = 1.0e-308_f64;
/// let zero = 0.0f64;
///
/// assert!(min.is_normal());
/// assert!(max.is_normal());
///
/// assert!(!zero.is_normal());
/// assert!(!f64::NAN.is_normal());
/// assert!(!f64::INFINITY.is_normal());
/// // Values between `0` and `min` are Subnormal.
/// assert!(!lower_than_min.is_normal());
/// ```
/// [subnormal]: https://en.wikipedia.org/wiki/Denormal_number
#[rune::function(instance)]
fn is_normal(this: f64) -> bool {
    this.is_normal()
}

/// Returns the maximum of the two numbers, ignoring NaN.
///
/// If one of the arguments is NaN, then the other argument is returned. This
/// follows the IEEE 754-2008 semantics for maxNum, except for handling of
/// signaling NaNs; this function handles all NaNs the same way and avoids
/// maxNum's problems with associativity. This also matches the behavior of
/// libm’s fmax.
///
/// # Examples
///
/// ```rune
/// let x = 1.0_f64;
/// let y = 2.0_f64;
///
/// assert_eq!(x.max(y), y);
/// ```
#[rune::function(instance)]
fn max(this: f64, other: f64) -> f64 {
    this.max(other)
}

/// Returns the minimum of the two numbers, ignoring NaN.
///
/// If one of the arguments is NaN, then the other argument is returned. This
/// follows the IEEE 754-2008 semantics for minNum, except for handling of
/// signaling NaNs; this function handles all NaNs the same way and avoids
/// minNum's problems with associativity. This also matches the behavior of
/// libm’s fmin.
///
/// # Examples
///
/// ```rune
/// let x = 1.0_f64;
/// let y = 2.0_f64;
///
/// assert_eq!(x.min(y), x);
/// ```
#[rune::function(instance)]
fn min(this: f64, other: f64) -> f64 {
    this.min(other)
}

/// Computes the absolute value of `self`.
///
/// # Examples
///
/// ```rune
/// let x = 3.5_f64;
/// let y = -3.5_f64;
///
/// let abs_difference_x = (x.abs() - x).abs();
/// let abs_difference_y = (y.abs() - (-y)).abs();
///
/// assert!(abs_difference_x < 1e-10);
/// assert!(abs_difference_y < 1e-10);
///
/// assert!(f64::NAN.abs().is_nan());
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn abs(this: f64) -> f64 {
    this.abs()
}

/// Raises a number to a floating point power.
///
/// # Examples
///
/// ```rune
/// let x = 2.0_f64;
/// let abs_difference = (x.powf(2.0) - (x * x)).abs();
///
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn powf(this: f64, other: f64) -> f64 {
    this.powf(other)
}

/// Raises a number to an integer power.
///
/// Using this function is generally faster than using `powf`. It might have a
/// different sequence of rounding operations than `powf`, so the results are
/// not guaranteed to agree.
///
/// # Examples
///
/// ```rune
/// let x = 2.0_f64;
/// let abs_difference = (x.powi(2) - (x * x)).abs();
///
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn powi(this: f64, other: i32) -> f64 {
    this.powi(other)
}

/// Test two floats for partial equality.
///
/// # Examples
///
/// ```rune
/// assert!(5.0 == 5.0);
/// assert!(5.0 != 10.0);
/// assert!(10.0 != 5.0);
/// assert!(10.0 != f64::NAN);
/// assert!(f64::NAN != f64::NAN);
/// ```
#[rune::function(instance, protocol = PARTIAL_EQ)]
#[inline]
fn partial_eq(this: f64, rhs: f64) -> bool {
    this.eq(&rhs)
}

/// Test two floats for total equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// assert_eq!(eq(5.0, 5.0), true);
/// assert_eq!(eq(5.0, 10.0), false);
/// assert_eq!(eq(10.0, 5.0), false);
/// ```
#[rune::function(instance, protocol = EQ)]
#[inline]
fn eq(this: f64, rhs: f64) -> VmResult<bool> {
    let Some(ordering) = this.partial_cmp(&rhs) else {
        return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: this, rhs });
    };

    VmResult::Ok(matches!(ordering, Ordering::Equal))
}

/// Perform a partial ordered comparison between two floats.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::partial_cmp;
///
/// assert_eq!(partial_cmp(5.0, 10.0), Some(Ordering::Less));
/// assert_eq!(partial_cmp(10.0, 5.0), Some(Ordering::Greater));
/// assert_eq!(partial_cmp(5.0, 5.0), Some(Ordering::Equal));
/// assert_eq!(partial_cmp(5.0, f64::NAN), None);
/// ```
#[rune::function(instance, protocol = PARTIAL_CMP)]
#[inline]
fn partial_cmp(this: f64, rhs: f64) -> Option<Ordering> {
    this.partial_cmp(&rhs)
}

/// Perform a partial ordered comparison between two floats.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::cmp;
///
/// assert_eq!(cmp(5.0, 10.0), Ordering::Less);
/// assert_eq!(cmp(10.0, 5.0), Ordering::Greater);
/// assert_eq!(cmp(5.0, 5.0), Ordering::Equal);
/// ```
#[rune::function(instance, protocol = CMP)]
#[inline]
fn cmp(this: f64, rhs: f64) -> VmResult<Ordering> {
    let Some(ordering) = this.partial_cmp(&rhs) else {
        return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: this, rhs });
    };

    VmResult::Ok(ordering)
}
