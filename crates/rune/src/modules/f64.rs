//! The `std::f64` module.

use core::num::ParseFloatError;

use crate as rune;
use crate::{ContextError, Module};

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["f64"]);

    module
        .function_meta(parse)?
        .deprecated("Use std::string::parse::<f64> instead");
    module.function_meta(is_nan)?;
    module.function_meta(max)?;
    module.function_meta(min)?;
    #[cfg(feature = "std")]
    module.function_meta(abs)?;
    #[cfg(feature = "std")]
    module.function_meta(powf)?;
    #[cfg(feature = "std")]
    module.function_meta(powi)?;
    module.function_meta(to_integer)?;
    module.constant(["NAN"], f64::NAN)?;
    Ok(module)
}

#[rune::function]
fn parse(s: &str) -> Result<f64, ParseFloatError> {
    str::parse::<f64>(s)
}

/// Convert a float to a whole number.
#[rune::function(instance)]
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

/// Returns the maximum of the two numbers, ignoring NaN.
///
/// If one of the arguments is NaN, then the other argument is returned. This
/// follows the IEEE 754-2008 semantics for maxNum, except for handling of
/// signaling NaNs; this function handles all NaNs the same way and avoids
/// maxNum's problems with associativity. This also matches the behavior of
/// libm’s fmax.
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
/// ```
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
