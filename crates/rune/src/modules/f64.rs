//! Floating point numbers.

use core::cmp::Ordering;
use core::num::ParseFloatError;

use crate as rune;
use crate::runtime::{VmError, VmErrorKind};
use crate::{docstring, ContextError, Module};

/// Mathematical constants mirroring Rust's [core::f64::consts].
pub mod consts {
    use crate as rune;
    use crate::{docstring, ContextError, Module};

    /// Mathematical constants mirroring Rust's [core::f64::consts].
    #[rune::module(::std::f64::consts)]
    pub fn module() -> Result<Module, ContextError> {
        let mut m = Module::from_meta(self::module__meta)?;

        m.constant("E", core::f64::consts::E)
            .build()?
            .docs(docstring!(
                /// Euler's number (e)
            ))?;

        m.constant("FRAC_1_PI", core::f64::consts::FRAC_1_PI)
            .build()?
            .docs(docstring!(
                /// 1 / π
            ))?;
        m.constant("FRAC_1_SQRT_2", core::f64::consts::FRAC_1_SQRT_2)
            .build()?
            .docs(docstring!(
                /// 1 / sqrt(2)
            ))?;
        m.constant("FRAC_2_PI", core::f64::consts::FRAC_2_PI)
            .build()?
            .docs(docstring!(
                /// 2 / π
            ))?;
        m.constant("FRAC_2_SQRT_PI", core::f64::consts::FRAC_2_SQRT_PI)
            .build()?
            .docs(docstring!(
                /// 2 / sqrt(π)
            ))?;

        m.constant("FRAC_PI_2", core::f64::consts::FRAC_PI_2)
            .build()?
            .docs(docstring!(
                /// π/2
            ))?;
        m.constant("FRAC_PI_3", core::f64::consts::FRAC_PI_3)
            .build()?
            .docs(docstring!(
                /// π/3
            ))?;
        m.constant("FRAC_PI_4", core::f64::consts::FRAC_PI_4)
            .build()?
            .docs(docstring!(
                /// π/4
            ))?;
        m.constant("FRAC_PI_6", core::f64::consts::FRAC_PI_6)
            .build()?
            .docs(docstring!(
                /// π/6
            ))?;
        m.constant("FRAC_PI_8", core::f64::consts::FRAC_PI_8)
            .build()?
            .docs(docstring!(
                /// π/8
            ))?;

        m.constant("LN_2", core::f64::consts::LN_2)
            .build()?
            .docs(docstring!(
                /// ln(2)
            ))?;
        m.constant("LN_10", core::f64::consts::LN_10)
            .build()?
            .docs(docstring!(
                /// ln(10)
            ))?;
        m.constant("LOG2_10", core::f64::consts::LOG2_10)
            .build()?
            .docs(docstring!(
                /// log<sub>2</sub>(10)
            ))?;
        m.constant("LOG2_E", core::f64::consts::LOG2_E)
            .build()?
            .docs(docstring!(
                /// log<sub>2</sub>(e)
            ))?;
        m.constant("LOG10_2", core::f64::consts::LOG10_2)
            .build()?
            .docs(docstring!(
                /// log<sub>10</sub>(2)
            ))?;
        m.constant("LOG10_E", core::f64::consts::LOG10_E)
            .build()?
            .docs(docstring!(
                /// log<sub>10</sub>(e)
            ))?;

        m.constant("PI", core::f64::consts::PI)
            .build()?
            .docs(docstring!(
                /// Archimede's constant (π)
            ))?;
        m.constant("SQRT_2", core::f64::consts::SQRT_2)
            .build()?
            .docs(docstring!(
                /// sqrt(2)
            ))?;
        m.constant("TAU", core::f64::consts::TAU)
            .build()?
            .docs(docstring!(
                /// The full circle constant (τ)
                ///
                /// Equal to 2π
            ))?;

        Ok(m)
    }
}

/// Floating point numbers.
///
/// This provides methods for computing over and parsing 64-bit floating pointer
/// numbers.
#[rune::module(::std::f64)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;

    m.function_meta(parse)?
        .deprecated("Use std::string::parse::<f64> instead")?;
    m.function_meta(is_nan)?;
    m.function_meta(is_infinite)?;
    m.function_meta(is_finite)?;
    m.function_meta(is_subnormal)?;
    m.function_meta(is_normal)?;
    m.function_meta(max__meta)?;
    m.function_meta(min__meta)?;

    #[cfg(feature = "std")]
    {
        m.function_meta(abs)?;
        m.function_meta(acos)?;
        m.function_meta(asin)?;
        m.function_meta(atan)?;
        m.function_meta(atan2)?;
        m.function_meta(cbrt)?;
        m.function_meta(ceil)?;
        m.function_meta(clamp)?;
        m.function_meta(cos)?;
        m.function_meta(div_euclid)?;
        m.function_meta(exp)?;
        m.function_meta(exp2)?;
        m.function_meta(floor)?;
        m.function_meta(ln)?;
        m.function_meta(log)?;
        m.function_meta(log10)?;
        m.function_meta(log2)?;
        m.function_meta(powf)?;
        m.function_meta(powi)?;
        m.function_meta(rem_euclid)?;
        m.function_meta(round)?;
        m.function_meta(sin)?;
        m.function_meta(sqrt)?;
        m.function_meta(tan)?;
    }
    m.function_meta(to_integer)?;
    m.function_meta(to_degrees)?;
    m.function_meta(to_radians)?;

    m.function_meta(clone__meta)?;
    m.implement_trait::<f64>(rune::item!(::std::clone::Clone))?;

    m.function_meta(partial_eq__meta)?;
    m.implement_trait::<f64>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(eq__meta)?;
    m.implement_trait::<f64>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(partial_cmp__meta)?;
    m.implement_trait::<f64>(rune::item!(::std::cmp::PartialOrd))?;

    m.function_meta(cmp__meta)?;
    m.implement_trait::<f64>(rune::item!(::std::cmp::Ord))?;

    m.constant("EPSILON", f64::EPSILON)
        .build()?
        .docs(docstring!(
            /// [Machine epsilon] value for `f64`.
            ///
            /// This is the difference between `1.0` and the next larger representable number.
            ///
            /// Equal to 2<sup>1 - MANTISSA_DIGITS</sup>.
            ///
            /// [Machine epsilon]: https://en.wikipedia.org/wiki/Machine_epsilon
        ))?;
    m.constant("MIN", f64::MIN).build()?.docs(docstring!(
        /// The smallest finite `f64` value.
        ///
        /// Equal to -[`MAX`].
        ///
        /// [`MAX`]: f64::MAX
    ))?;
    m.constant("MAX", f64::MAX).build()?.docs(docstring!(
        /// Largest finite `f64` value.
        ///
        /// Equal to
        /// (1 - 2<sup>-MANTISSA_DIGITS</sup>) 2<sup>[`MAX_EXP`]</sup>.
        ///
        /// [`MAX_EXP`]: f64::MAX_EXP
    ))?;
    m.constant("MIN_POSITIVE", f64::MIN_POSITIVE)
        .build()?
        .docs(docstring!(
            /// Smallest positive normal `f64` value.
            ///
            /// Equal to 2<sup>[`MIN_EXP`] - 1</sup>.
            ///
            /// [`MIN_EXP`]: f64::MIN_EXP
        ))?;
    m.constant("MIN_EXP", f64::MIN_EXP)
        .build()?
        .docs(docstring!(
            /// One greater than the minimum possible *normal* power of 2 exponent
            /// for a significand bounded by 1 ≤ x < 2 (i.e. the IEEE definition).
            ///
            /// This corresponds to the exact minimum possible *normal* power of 2 exponent
            /// for a significand bounded by 0.5 ≤ x < 1 (i.e. the C definition).
            /// In other words, all normal numbers representable by this type are
            /// greater than or equal to 0.5 × 2<sup><i>MIN_EXP</i></sup>.
        ))?;
    m.constant("MAX_EXP", f64::MAX_EXP)
        .build()?
        .docs(docstring!(
            /// One greater than the maximum possible power of 2 exponent
            /// for a significand bounded by 1 ≤ x < 2 (i.e. the IEEE definition).
            ///
            /// This corresponds to the exact maximum possible power of 2 exponent
            /// for a significand bounded by 0.5 ≤ x < 1 (i.e. the C definition).
            /// In other words, all numbers representable by this type are
            /// strictly less than 2<sup><i>MAX_EXP</i></sup>.
        ))?;
    m.constant("MIN_10_EXP", f64::MIN_10_EXP)
        .build()?
        .docs(docstring!(
            /// Minimum <i>x</i> for which 10<sup><i>x</i></sup> is normal.
            ///
            /// Equal to ceil(log<sub>10</sub> [`MIN_POSITIVE`]).
            ///
            /// [`MIN_POSITIVE`]: f64::MIN_POSITIVE
        ))?;
    m.constant("MAX_10_EXP", f64::MAX_10_EXP)
        .build()?
        .docs(docstring!(
            /// Maximum <i>x</i> for which 10<sup><i>x</i></sup> is normal.
            ///
            /// Equal to floor(log<sub>10</sub> [`MAX`]).
            ///
            /// [`MAX`]: f64::MAX
        ))?;
    m.constant("NAN", f64::NAN).build()?.docs(docstring!(
        /// Not a number (NaN).
        ///
        ///
        /// Note that IEEE 754 doesn't define just a single NaN value; a plethora of bit patterns
        /// are considered to be NaN. Furthermore, the standard makes a difference between a
        /// "signaling" and a "quiet" NaN, and allows inspecting its "payload" (the unspecified
        /// bits in the bit pattern) and its sign. See the [Rust documentation of NaN bit
        /// patterns](https://doc.rust-lang.org/core/primitive.f32.html#nan-bit-patterns) for more
        /// info.
        ///
        /// This constant is guaranteed to be a quiet NaN (on targets that follow the Rust assumptions
        /// that the quiet/signaling bit being set to 1 indicates a quiet NaN). Beyond that, nothing is
        /// guaranteed about the specific bit pattern chosen here: both payload and sign are arbitrary.
        /// The concrete bit pattern may change across Rust versions and target platforms.
    ))?;
    m.constant("INFINITY", f64::INFINITY)
        .build()?
        .docs(docstring!(
            /// Positive infinity (∞).
        ))?;
    m.constant("NEG_INFINITY", f64::NEG_INFINITY)
        .build()?
        .docs(docstring!(
            /// Negative infinity (−∞).
        ))?;

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

/// Converts radians to degrees.
///
/// # Examples
///
/// ```rune
/// let abs_difference = (std::f64::consts::PI.to_degrees() - 180.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
fn to_degrees(this: f64) -> f64 {
    this.to_degrees()
}

/// Converts degrees to radians.
///
/// # Examples
///
/// ```rune
/// let abs_difference = (180.0.to_radians() - std::f64::consts::PI).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
fn to_radians(this: f64) -> f64 {
    this.to_radians()
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
#[rune::function(keep, instance, protocol = MAX)]
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
#[rune::function(keep, instance, protocol = MIN)]
fn min(this: f64, other: f64) -> f64 {
    this.min(other)
}

/// Returns the square root of a number.
///
/// Returns NaN if `self` is a negative number other than `-0.0`.
///
/// # Examples
///
/// ```rune
/// let positive = 4.0_f64;
/// let negative = -4.0_f64;
/// let negative_zero = -0.0_f64;
///
/// let abs_difference = (positive.sqrt() - 2.0).abs();
///
/// assert!(abs_difference < 1e-10);
/// assert!(negative.sqrt().is_nan());
/// assert!(negative_zero.sqrt() == negative_zero);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn sqrt(this: f64) -> f64 {
    this.sqrt()
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

/// Returns the largest integer less than or equal to `self`.
///
/// # Examples
///
/// ```rune
/// let f = 3.7_f64;
/// let g = 3.0_f64;
/// let h = -3.7_f64;
///
/// assert!(f.floor() == 3.0);
/// assert!(g.floor() == 3.0);
/// assert!(h.floor() == -4.0);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn floor(this: f64) -> f64 {
    this.floor()
}

/// Returns the smallest integer greater than or equal to `self`.
///
/// # Examples
///
/// ```rune
/// let f = 3.01_f64;
/// let g = 4.0_f64;
///
/// assert_eq!(f.ceil(), 4.0);
/// assert_eq!(g.ceil(), 4.0);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn ceil(this: f64) -> f64 {
    this.ceil()
}

/// Returns the nearest integer to `self`. If a value is half-way between two
/// integers, round away from `0.0`.
///
/// # Examples
///
/// ```rune
/// let f = 3.3_f64;
/// let g = -3.3_f64;
/// let h = -3.7_f64;
/// let i = 3.5_f64;
/// let j = 4.5_f64;
///
/// assert_eq!(f.round(), 3.0);
/// assert_eq!(g.round(), -3.0);
/// assert_eq!(h.round(), -4.0);
/// assert_eq!(i.round(), 4.0);
/// assert_eq!(j.round(), 5.0);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn round(this: f64) -> f64 {
    this.round()
}

/// Clone a `f64`.
///
/// Note that since the type is copy, cloning has the same effect as assigning
/// it.
///
/// # Examples
///
/// ```rune
/// let a = 5.0;
/// let b = a;
/// let c = a.clone();
///
/// a += 1.0;
///
/// assert_eq!(a, 6.0);
/// assert_eq!(b, 5.0);
/// assert_eq!(c, 5.0);
/// ```
#[rune::function(keep, instance, protocol = CLONE)]
#[inline]
fn clone(this: f64) -> f64 {
    this
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
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
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
#[rune::function(keep, instance, protocol = EQ)]
#[inline]
fn eq(this: f64, rhs: f64) -> Result<bool, VmError> {
    let Some(ordering) = this.partial_cmp(&rhs) else {
        return Err(VmError::new(VmErrorKind::IllegalFloatComparison {
            lhs: this,
            rhs,
        }));
    };

    Ok(matches!(ordering, Ordering::Equal))
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
#[rune::function(keep, instance, protocol = PARTIAL_CMP)]
#[inline]
fn partial_cmp(this: f64, rhs: f64) -> Option<Ordering> {
    this.partial_cmp(&rhs)
}

/// Perform a totally ordered comparison between two floats.
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
#[rune::function(keep, instance, protocol = CMP)]
#[inline]
fn cmp(this: f64, rhs: f64) -> Result<Ordering, VmError> {
    let Some(ordering) = this.partial_cmp(&rhs) else {
        return Err(VmError::new(VmErrorKind::IllegalFloatComparison {
            lhs: this,
            rhs,
        }));
    };

    Ok(ordering)
}

/// Computes the arccosine of a number.
///
/// Return value is in radians in the range [0, pi] or NaN if the number is outside the range [-1,
/// 1].
///
/// # Examples
///
/// ```rune
/// let f = std::f64::consts::FRAC_PI_4;
///
/// // acos(cos(pi/4))
/// let abs_difference = (f.cos().acos() - std::f64::consts::FRAC_PI_4).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn acos(this: f64) -> f64 {
    this.acos()
}

/// Computes the arcsine of a number.
///
/// Return value is in radians in the range [-pi/2, pi/2] or NaN if the number is outside the range
/// [-1, 1].
///
/// # Examples
///
/// ```rune
/// let f = std::f64::consts::FRAC_PI_2;
///
/// // asin(sin(pi/2))
/// let abs_difference = (f.sin().asin() - std::f64::consts::FRAC_PI_2).abs();
/// assert!(abs_difference < 1e-7);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn asin(this: f64) -> f64 {
    this.asin()
}

/// Computes the arctangent of a number.
///
/// Return value is in radians in the range [-pi/2, pi/2];
///
/// # Examples
///
/// ```rune
/// let f = 1.0;
///
/// // atan(tan(1))
/// let abs_difference = (f.tan().atan() - 1.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn atan(this: f64) -> f64 {
    this.atan()
}

/// Computes the four quadrant arctangent of self (y) and other (x) in radians.
///
/// * `x = 0`, `y = 0`: `0`
/// * `x >= 0`: `arctan(y/x)` -> `[-pi/2, pi/2]`
/// * `y >= 0`: `arctan(y/x) + pi` -> `(pi/2, pi]`
/// * `y < 0`: `arctan(y/x) - pi` -> `(-pi, -pi/2)`
///
/// # Examples
///
/// ```rune
/// // Positive angles measured counter-clockwise
/// // from positive x axis
/// // -pi/4 radians (45 deg clockwise)
/// let x1 = 3.0;
/// let y1 = -3.0;
///
/// // 3pi/4 radians (135 deg counter-clockwise)
/// let x2 = -3.0;
/// let y2 = 3.0;
///
/// let abs_difference_1 = (y1.atan2(x1) - (-std::f64::consts::FRAC_PI_4)).abs();
/// let abs_difference_2 = (y2.atan2(x2) - (3.0 * std::f64::consts::FRAC_PI_4)).abs();
///
/// assert!(abs_difference_1 < 1e-10);
/// assert!(abs_difference_2 < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn atan2(this: f64, other: f64) -> f64 {
    this.atan2(other)
}

/// Returns the cube root of a number.
///
/// # Examples
///
/// ```rune
/// let x = 8.0_f64;
///
/// // x^(1/3) - 2 == 0
/// let abs_difference = (x.cbrt() - 2.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn cbrt(this: f64) -> f64 {
    this.cbrt()
}

/// Computes the cosine of a number (in radians).
///
/// # Examples
///
/// ```rune
/// let x = 2.0 * std::f64::consts::PI;
///
/// let abs_difference = (x.cos() - 1.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn cos(this: f64) -> f64 {
    this.cos()
}

/// Restrict a value to a certain interval unless it is NaN.
///
/// Returns `max` if `self` is greater than `max`, and `min` if `self` is less than `min`.
/// Otherwise this returns `self`.
///
/// Note that this function returns NaN if the initial value was NaN as well.
///
/// # Panics
///
/// Panics if `min > max`, `min` is NaN, or `max` is NaN.
///
/// # Examples
///
/// ```rune
/// assert!((-3.0f64).clamp(-2.0, 1.0) == -2.0);
/// assert!((0.0f64).clamp(-2.0, 1.0) == 0.0);
/// assert!((2.0f64).clamp(-2.0, 1.0) == 1.0);
/// assert!((f64::NAN).clamp(-2.0, 1.0).is_nan());
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn clamp(this: f64, min: f64, max: f64) -> f64 {
    this.clamp(min, max)
}

/// Calculates Euclidean division, the matching method for rem_euclid.
///
/// This computes the integer `n` such that `self = n * rhs + self.rem_euclid(rhs)`. In other
/// words, the result is `self / rhs` rounded to the integer `n` such that `self >= n * rhs`.
///
/// # Examples
///
/// ```rune
/// let a = 7.0;
/// let b = 4.0;
/// assert_eq!(a.div_euclid(b), 1.0); // 7.0 > 4.0 * 1.0
/// assert_eq!((-a).div_euclid(b), -2.0); // -7.0 >= 4.0 * -2.0
/// assert_eq!(a.div_euclid(-b), -1.0); // 7.0 >= -4.0 * -1.0
/// assert_eq!((-a).div_euclid(-b), 2.0); // -7.0 >= -4.0 * 2.0
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn div_euclid(this: f64, rhs: f64) -> f64 {
    this.div_euclid(rhs)
}

/// Computes the least nonnegative remainder of `self (mod rhs)`.
///
/// In particular, the return value `r` satisfies `0.0 <= r < rhs.abs()` in most cases. However,
/// due to a floating point round-off error it can result in `r == rhs.abs()`, violating the
/// mathematical definition, if `self` is much smaller than `rhs.abs()` in magnitude and `self <
/// 0.0`. This result is not an element of the function’s codomain, but it is the closest floating
/// point number in the real numbers and thus fulfills the property `self == self.div_euclid(rhs) *
/// rhs + self.rem_euclid(rhs)` approximately.
///
/// # Examples
///
/// ```rune
/// let a = 7.0;
/// let b = 4.0;
/// assert_eq!(a.rem_euclid(b), 3.0);
/// assert_eq!((-a).rem_euclid(b), 1.0);
/// assert_eq!(a.rem_euclid(-b), 3.0);
/// assert_eq!((-a).rem_euclid(-b), 1.0);
/// // limitation due to round-off error
/// assert!((-f64::EPSILON).rem_euclid(3.0) != 0.0);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn rem_euclid(this: f64, rhs: f64) -> f64 {
    this.rem_euclid(rhs)
}

/// Returns `e^(self)`, (the exponential function).
///
/// # Examples
///
/// ```rune
/// let one = 1.0_f64;
/// // e^1
/// let e = one.exp();
///
/// // ln(e) - 1 == 0
/// let abs_difference = (e.ln() - 1.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn exp(this: f64) -> f64 {
    this.exp()
}

/// Returns `2^(self)`.
///
/// Examples
///
/// ```rune
/// let f = 2.0_f64;
///
/// // 2^2 - 4 == 0
/// let abs_difference = (f.exp2() - 4.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn exp2(this: f64) -> f64 {
    this.exp2()
}

/// Returns the natural logarithm of the number.
///
/// This returns NaN when the number is negative, and negative infinity when number is zero.
/// let one = 1.0_f64;
///
/// # Examples
///
/// ```rune
/// // e^1
/// let e = one.exp();
///
/// // ln(e) - 1 == 0
/// let abs_difference = (e.ln() - 1.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn ln(this: f64) -> f64 {
    this.ln()
}

/// Returns the logarithm of the number with respect to an arbitrary base.
///
/// This returns NaN when the number is negative, and negative infinity when number is zero.
///
/// The result might not be correctly rounded owing to implementation details; `self.log2()` can
/// produce more accurate results for base 2, and `self.log10()` can produce more accurate results
/// for base 10.
///
/// # Examples
///
/// ```rune
/// let twenty_five = 25.0_f64;
///
/// // log5(25) - 2 == 0
/// let abs_difference = (twenty_five.log(5.0) - 2.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn log(this: f64, base: f64) -> f64 {
    this.log(base)
}

/// Returns the base 2 logarithm of the number.
///
/// This returns NaN when the number is negative, and negative infinity when number is zero.
///
/// # Examples
///
/// ```rune
/// let four = 4.0_f64;
///
/// // log2(4) - 2 == 0
/// let abs_difference = (four.log2() - 2.0).abs();
///
/// assert!(abs_difference < 1e-10);
/// ```
///
/// Non-positive values:
///
/// ```rune
/// assert_eq!(0_f64.log2(), f64::NEG_INFINITY);
/// assert!((-42_f64).log2().is_nan());
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn log2(this: f64) -> f64 {
    this.log2()
}

/// Returns the base 10 logarithm of the number.
///
/// This returns NaN when the number is negative, and negative infinity when number is zero.
///
/// # Examples
///
/// ```rune
/// let hundred = 100.0_f64;
///
/// // log10(100) - 2 == 0
/// let abs_difference = (hundred.log10() - 2.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn log10(this: f64) -> f64 {
    this.log10()
}

/// Computes the sine of a number (in radians).
///
/// # Examples
///
/// ```rune
/// let x = std::f64::consts::FRAC_PI_2;
///
/// let abs_difference = (x.sin() - 1.0).abs();
/// assert!(abs_difference < 1e-10);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn sin(this: f64) -> f64 {
    this.sin()
}

/// Computes the tangent of a number (in radians).
///
/// # Examples
///
/// ```rune
/// let x = std::f64::consts::FRAC_PI_4;
/// let abs_difference = (x.tan() - 1.0).abs();
/// assert!(abs_difference < 1e-14);
/// ```
#[rune::function(instance)]
#[cfg(feature = "std")]
fn tan(this: f64) -> f64 {
    this.tan()
}
