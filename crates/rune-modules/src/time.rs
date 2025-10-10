//! The native `time` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.14.0", features = ["time"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::time::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use time;
//!
//! fn main() {
//!     time::sleep(time::Duration::from_secs(10)).await;
//!     println("Message after 10 seconds!");
//! }
//! ```

use core::cmp::Ordering;
use core::hash::Hash;

use rune::alloc;
use rune::alloc::fmt::TryWrite;
use rune::runtime::{Formatter, Hasher, Mut, VmError};
use rune::{docstring, item, Any, ContextError, Module, ToConstValue};

const NANOS_PER_SEC: u32 = 1_000_000_000;

/// Construct the `time` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut m = Module::with_crate("time")?;

    m.function_meta(sleep)?;
    m.function_meta(interval)?;
    m.function_meta(interval_at)?;

    m.ty::<Duration>()?;

    m.function_meta(Duration::new__meta)?;
    m.function_meta(Duration::from_secs__meta)?;
    m.function_meta(Duration::from_millis__meta)?;
    m.function_meta(Duration::from_micros__meta)?;
    m.function_meta(Duration::from_nanos__meta)?;
    m.function_meta(Duration::is_zero__meta)?;
    m.function_meta(Duration::as_secs__meta)?;
    m.function_meta(Duration::subsec_millis__meta)?;
    m.function_meta(Duration::subsec_micros__meta)?;
    m.function_meta(Duration::subsec_nanos__meta)?;
    m.function_meta(Duration::as_millis__meta)?;
    m.function_meta(Duration::as_micros__meta)?;
    m.function_meta(Duration::as_nanos__meta)?;
    m.function_meta(Duration::as_secs_f64__meta)?;
    m.function_meta(Duration::from_secs_f64__meta)?;
    m.function_meta(Duration::add__meta)?;
    m.function_meta(Duration::add_assign__meta)?;
    m.function_meta(Duration::partial_eq__meta)?;
    m.implement_trait::<Duration>(item!(::std::cmp::PartialEq))?;
    m.function_meta(Duration::eq__meta)?;
    m.implement_trait::<Duration>(item!(::std::cmp::Eq))?;
    m.function_meta(Duration::partial_cmp__meta)?;
    m.implement_trait::<Duration>(item!(::std::cmp::PartialOrd))?;
    m.function_meta(Duration::cmp__meta)?;
    m.implement_trait::<Duration>(item!(::std::cmp::Ord))?;
    m.function_meta(Duration::hash__meta)?;
    m.function_meta(Duration::debug_fmt__meta)?;
    m.function_meta(Duration::clone__meta)?;
    m.implement_trait::<Duration>(item!(::std::clone::Clone))?;

    m.constant(
        "SECOND",
        Duration {
            inner: tokio::time::Duration::from_secs(1),
        },
    )
    .build_associated::<Duration>()?
    .docs(docstring! {
        /// The duration of one second.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use time::Duration;
        ///
        /// let duration = Duration::SECOND;
        /// ```
    })?;

    m.constant(
        "MILLISECOND",
        Duration {
            inner: tokio::time::Duration::from_millis(1),
        },
    )
    .build_associated::<Duration>()?
    .docs(docstring! {
        /// The duration of one millisecond.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use time::Duration;
        ///
        /// let duration = Duration::MILLISECOND;
        /// ```
    })?;

    m.constant(
        "MICROSECOND",
        Duration {
            inner: tokio::time::Duration::from_micros(1),
        },
    )
    .build_associated::<Duration>()?
    .docs(docstring! {
        /// The duration of one microsecond.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use time::Duration;
        ///
        /// let duration = Duration::MICROSECOND;
        /// ```
    })?;

    m.constant(
        "NANOSECOND",
        Duration {
            inner: tokio::time::Duration::from_nanos(1),
        },
    )
    .build_associated::<Duration>()?
    .docs(docstring! {
        /// The duration of one nanosecond.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use time::Duration;
        ///
        /// let duration = Duration::NANOSECOND;
        /// ```
    })?;

    m.constant(
        "ZERO",
        Duration {
            inner: tokio::time::Duration::ZERO,
        },
    )
    .build_associated::<Duration>()?
    .docs(docstring! {
        /// A duration of zero time.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use time::Duration;
        ///
        /// let duration = Duration::ZERO;
        /// ```
    })?;

    m.constant(
        "MAX",
        Duration {
            inner: tokio::time::Duration::MAX,
        },
    )
    .build_associated::<Duration>()?
    .docs(docstring! {
        /// The maximum duration.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use time::Duration;
        ///
        /// let duration = Duration::MAX;
        /// assert!(Duration::ZERO < Duration::MAX);
        /// ```
    })?;

    m.ty::<Instant>()?;
    m.function_meta(Instant::now__meta)?;
    m.function_meta(Instant::duration_since__meta)?;
    m.function_meta(Instant::elapsed__meta)?;
    m.function_meta(Instant::add__meta)?;
    m.function_meta(Instant::add_assign__meta)?;
    m.function_meta(Instant::sub__meta)?;
    m.function_meta(Instant::sub_assign__meta)?;
    m.function_meta(Instant::sub_instant__meta)?;
    m.function_meta(Instant::sub_instant_assign__meta)?;
    m.function_meta(Instant::partial_eq__meta)?;
    m.implement_trait::<Instant>(item!(::std::cmp::PartialEq))?;
    m.function_meta(Instant::eq__meta)?;
    m.implement_trait::<Instant>(item!(::std::cmp::Eq))?;
    m.function_meta(Instant::partial_cmp__meta)?;
    m.implement_trait::<Instant>(item!(::std::cmp::PartialOrd))?;
    m.function_meta(Instant::cmp__meta)?;
    m.implement_trait::<Instant>(item!(::std::cmp::Ord))?;
    m.function_meta(Instant::hash__meta)?;
    m.function_meta(Instant::debug_fmt__meta)?;
    m.function_meta(Instant::clone__meta)?;
    m.implement_trait::<Instant>(item!(::std::clone::Clone))?;

    m.ty::<Interval>()?;
    m.function("tick", Interval::tick)
        .build_associated::<Interval>()?;
    m.function_meta(Interval::reset__meta)?;
    m.function_meta(Interval::reset_immediately__meta)?;
    m.function_meta(Interval::reset_after__meta)?;
    m.function_meta(Interval::reset_at__meta)?;

    Ok(m)
}

/// Waits until duration has elapsed.
///
/// # Examples
///
/// ```rune,no_run
/// use time::Duration;
///
/// let duration = Duration::from_secs(10);
/// time::sleep(duration).await;
/// println!("Surprise!");
/// ```
#[rune::function]
async fn sleep(duration: Duration) {
    tokio::time::sleep(duration.inner).await;
}

/// Creates new [`Interval`] that yields with interval of `period`. The first
/// tick completes immediately.
///
/// An interval will tick indefinitely. At any time, the [`Interval`] value can
/// be dropped. This cancels the interval.
///
/// # Examples
///
/// ```rune,no_run
/// use time::Duration;
///
/// let duration = Duration::from_millis(10);
/// let interval = time::interval(duration);
///
/// interval.tick().await; // ticks immediately
/// interval.tick().await; // ticks after 10ms
/// interval.tick().await; // ticks after 10ms
///
/// println!("approximately 20ms have elapsed...");
/// ```
#[rune::function]
async fn interval(period: Duration) -> Interval {
    Interval {
        inner: tokio::time::interval(period.inner),
    }
}

/// Creates new [`Interval`] that yields with interval of `period` with the
/// first tick completing at `start`.
///
/// An interval will tick indefinitely. At any time, the [`Interval`] value can
/// be dropped. This cancels the interval.
///
/// # Vm Panics
///
/// This function panics if `period` is zero.
///
/// # Examples
///
/// ```rune,no_run
/// use time::{Duration, Instant};
///
/// let start = Instant::now() + Duration::from_millis(50);
/// let interval = time::interval_at(start, Duration::from_millis(10));
///
/// interval.tick().await; // ticks after 50ms
/// interval.tick().await; // ticks after 10ms
/// interval.tick().await; // ticks after 10ms
///
/// println!("approximately 70ms have elapsed...");
/// ```
#[rune::function]
async fn interval_at(start: Instant, period: Duration) -> Interval {
    Interval {
        inner: tokio::time::interval_at(start.inner, period.inner),
    }
}

/// A `Duration` type to represent a span of time, typically used for system
/// timeouts.
///
/// Each `Duration` is composed of a whole number of seconds and a fractional part
/// represented in nanoseconds. If the underlying system does not support
/// nanosecond-level precision, APIs binding a system timeout will typically round up
/// the number of nanoseconds.
///
/// # Examples
///
/// ```rune
/// use time::Duration;
///
/// let five_seconds = Duration::new(5, 0);
/// let five_seconds_and_five_nanos = five_seconds + Duration::new(0, 5);
///
/// assert_eq!(five_seconds_and_five_nanos.as_secs(), 5);
/// assert_eq!(five_seconds_and_five_nanos.subsec_nanos(), 5);
///
/// let ten_millis = Duration::from_millis(10);
/// ```
#[derive(Debug, Clone, Copy, Any, ToConstValue)]
#[rune(item = ::time)]
pub struct Duration {
    #[const_value(with = self::const_duration)]
    inner: tokio::time::Duration,
}

impl Duration {
    /// Converts [`Duration`] into a [`std::time::Duration`].
    pub fn into_std(self) -> std::time::Duration {
        std::time::Duration::new(self.inner.as_secs(), self.inner.subsec_nanos())
    }

    /// Creates a [`Duration`] from a [`std::time::Duration`].
    pub fn from_std(duration: std::time::Duration) -> Self {
        Self {
            inner: tokio::time::Duration::new(duration.as_secs(), duration.subsec_nanos()),
        }
    }

    /// Converts [`Duration`] into a [`tokio::time::Duration`].
    ///
    /// # Example
    ///
    /// ```
    /// use rune_modules::time::Duration;
    ///
    /// let duration = Duration::from_secs(5);
    /// let tokio_duration = duration.into_tokio();
    /// ```
    pub fn into_tokio(self) -> tokio::time::Duration {
        self.inner
    }

    /// Creates a [`Duration`] from a [`tokio::time::Duration`].
    ///
    /// # Example
    ///
    /// ```
    /// use rune_modules::time::Duration;
    ///
    /// let tokio_duration = tokio::time::Duration::from_secs(5);
    /// let duration = Duration::from_tokio(tokio_duration);
    /// ```
    pub fn from_tokio(duration: tokio::time::Duration) -> Self {
        Self { inner: duration }
    }

    /// Creates a new `Duration` from the specified number of whole seconds and
    /// additional nanoseconds.
    ///
    /// If the number of nanoseconds is greater than 1 billion (the number of
    /// nanoseconds in a second), then it will carry over into the seconds provided.
    ///
    /// # Vm Panics
    ///
    /// This constructor will panic if the carry from the nanoseconds overflows
    /// the seconds counter.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let five_seconds = Duration::new(5, 0);
    /// ```
    #[rune::function(keep, path = Self::new)]
    pub fn new(secs: u64, nanos: u32) -> Result<Self, VmError> {
        if nanos >= NANOS_PER_SEC && secs.checked_add((nanos / NANOS_PER_SEC) as u64).is_none() {
            return Err(VmError::panic("overflow in Duration::new"));
        }

        Ok(Self {
            inner: tokio::time::Duration::new(secs, nanos),
        })
    }

    /// Creates a new `Duration` from the specified number of whole seconds.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_secs(5);
    /// ```
    #[rune::function(keep, path = Self::from_secs)]
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_secs(secs),
        }
    }

    /// Creates a new `Duration` from the specified number of milliseconds.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_millis(2569);
    /// ```
    #[rune::function(keep, path = Self::from_millis)]
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_millis(millis),
        }
    }

    /// Creates a new `Duration` from the specified number of microseconds.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_micros(1_000_002);
    /// ```
    #[rune::function(keep, path = Self::from_micros)]
    #[inline]
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_micros(micros),
        }
    }

    /// Creates a new `Duration` from the specified number of nanoseconds.
    ///
    /// Note: Using this on the return value of `as_nanos()` might cause unexpected behavior:
    /// `as_nanos()` returns a u128, and can return values that do not fit in u64, e.g. 585 years.
    /// Instead, consider using the pattern `Duration::new(d.as_secs(), d.subsec_nanos())`
    /// if you cannot copy/clone the Duration directly.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_nanos(1_000_000_123);
    /// ```
    #[rune::function(keep, path = Self::from_nanos)]
    #[inline]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_nanos(nanos),
        }
    }

    /// Returns true if this `Duration` spans no time.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// assert!(Duration::ZERO.is_zero());
    /// assert!(Duration::new(0, 0).is_zero());
    /// assert!(Duration::from_nanos(0).is_zero());
    /// assert!(Duration::from_secs(0).is_zero());
    ///
    /// assert!(!Duration::new(1, 1).is_zero());
    /// assert!(!Duration::from_nanos(1).is_zero());
    /// assert!(!Duration::from_secs(1).is_zero());
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    /// Returns the number of _whole_ seconds contained by this `Duration`.
    ///
    /// The returned value does not include the fractional (nanosecond) part of
    /// the duration, which can be obtained using [`subsec_nanos`].
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::new(5, 730023852);
    /// assert_eq!(duration.as_secs(), 5);
    /// ```
    ///
    /// To determine the total number of seconds represented by the `Duration`
    /// including the fractional part, use [`as_secs_f64`] or [`as_secs_f32`]
    ///
    /// [`as_secs_f64`]: Duration::as_secs_f64
    /// [`as_secs_f32`]: Duration::as_secs_f32
    /// [`subsec_nanos`]: Duration::subsec_nanos
    #[rune::function(keep)]
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        self.inner.as_secs()
    }

    /// Returns the fractional part of this `Duration`, in whole milliseconds.
    ///
    /// This method does **not** return the length of the duration when
    /// represented by milliseconds. The returned number always represents a
    /// fractional portion of a second (i.e., it is less than one thousand).
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_millis(5432);
    /// assert_eq!(duration.as_secs(), 5);
    /// assert_eq!(duration.subsec_millis(), 432);
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn subsec_millis(&self) -> u32 {
        self.inner.subsec_millis()
    }

    /// Returns the fractional part of this `Duration`, in whole microseconds.
    ///
    /// This method does **not** return the length of the duration when
    /// represented by microseconds. The returned number always represents a
    /// fractional portion of a second (i.e., it is less than one million).
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_micros(1_234_567);
    /// assert_eq!(duration.as_secs(), 1);
    /// assert_eq!(duration.subsec_micros(), 234_567);
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn subsec_micros(&self) -> u32 {
        self.inner.subsec_micros()
    }

    /// Returns the fractional part of this `Duration`, in nanoseconds.
    ///
    /// This method does **not** return the length of the duration when
    /// represented by nanoseconds. The returned number always represents a
    /// fractional portion of a second (i.e., it is less than one billion).
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_millis(5010);
    /// assert_eq!(duration.as_secs(), 5);
    /// assert_eq!(duration.subsec_nanos(), 10_000_000);
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn subsec_nanos(&self) -> u32 {
        self.inner.subsec_nanos()
    }

    /// Returns the total number of whole milliseconds contained by this
    /// `Duration`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::new(5, 730023852);
    /// assert_eq!(duration.as_millis(), 5730);
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn as_millis(&self) -> u128 {
        self.inner.as_millis()
    }

    /// Returns the total number of whole microseconds contained by this
    /// `Duration`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::new(5, 730023852);
    /// assert_eq!(duration.as_micros(), 5730023);
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn as_micros(&self) -> u128 {
        self.inner.as_micros()
    }

    /// Returns the total number of nanoseconds contained by this `Duration`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::new(5, 730023852);
    /// assert_eq!(duration.as_nanos(), 5730023852);
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub const fn as_nanos(&self) -> u128 {
        self.inner.as_nanos()
    }

    /// Returns the number of seconds contained by this `Duration` as `f64`.
    ///
    /// The returned value does include the fractional (nanosecond) part of the
    /// duration.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_secs(60).as_secs_f64();
    /// ```
    #[rune::function(keep)]
    #[inline]
    pub fn as_secs_f64(&self) -> f64 {
        self.inner.as_secs_f64()
    }

    /// Creates a new `Duration` from the specified number of seconds represented
    /// as `f64`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let duration = Duration::from_secs_f64(0.0);
    /// ```
    #[rune::function(keep, path = Self::from_secs_f64)]
    pub fn from_secs_f64(secs: f64) -> Result<Self, VmError> {
        match tokio::time::Duration::try_from_secs_f64(secs) {
            Ok(duration) => Ok(Self { inner: duration }),
            Err(e) => Err(VmError::panic(e)),
        }
    }

    /// Add a duration to this instant and return a new instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let first = Duration::SECOND;
    /// let second = first + Duration::SECOND;
    ///
    /// assert!(first < second);
    /// ```
    #[rune::function(keep, instance, protocol = ADD)]
    #[inline]
    fn add(&self, rhs: &Duration) -> Result<Self, VmError> {
        let Some(inner) = self.inner.checked_add(rhs.inner) else {
            return Err(VmError::panic("overflow when adding durations"));
        };

        Ok(Self { inner })
    }

    /// Add a duration to this instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let first = Duration::SECOND;
    /// let second = first.clone();
    /// second += Duration::SECOND;
    ///
    /// assert!(first < second);
    /// ```
    #[rune::function(keep, instance, protocol = ADD_ASSIGN)]
    #[inline]
    fn add_assign(&mut self, rhs: &Duration) -> Result<(), VmError> {
        let Some(inner) = self.inner.checked_add(rhs.inner) else {
            return Err(VmError::panic("overflow when adding duration to instant"));
        };

        self.inner = inner;
        Ok(())
    }

    /// Test two durations for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::partial_eq;
    ///
    /// use time::Duration;
    ///
    /// let millis = Duration::MILLISECOND;
    /// let second = Duration::SECOND;
    ///
    /// assert_eq!(partial_eq(millis, millis), true);
    /// assert_eq!(partial_eq(millis, second), false);
    /// assert_eq!(partial_eq(second, millis), false);
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_EQ)]
    #[inline]
    fn partial_eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Test two durations for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// use time::Duration;
    ///
    /// let millis = Duration::MILLISECOND;
    /// let second = Duration::SECOND;
    ///
    /// assert_eq!(eq(millis, millis), true);
    /// assert_eq!(eq(millis, second), false);
    /// assert_eq!(eq(second, millis), false);
    /// ```
    #[rune::function(keep, instance, protocol = EQ)]
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Perform a partial ordered comparison between two durations.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let millis = Duration::MILLISECOND;
    /// let second = Duration::SECOND;
    ///
    /// assert!(millis < second);
    /// assert!(second > millis);
    /// assert!(millis == millis);
    /// ```
    ///
    /// Using explicit functions:
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::partial_cmp;
    ///
    /// use time::Duration;
    ///
    /// let millis = Duration::MILLISECOND;
    /// let second = Duration::SECOND;
    ///
    /// assert_eq!(partial_cmp(millis, second), Some(Ordering::Less));
    /// assert_eq!(partial_cmp(second, millis), Some(Ordering::Greater));
    /// assert_eq!(partial_cmp(millis, millis), Some(Ordering::Equal));
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_CMP)]
    #[inline]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.inner, &rhs.inner)
    }

    /// Perform a totally ordered comparison between two durations.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::cmp;
    ///
    /// use time::Duration;
    ///
    /// let millis = Duration::MILLISECOND;
    /// let second = Duration::SECOND;
    ///
    /// assert_eq!(cmp(millis, second), Ordering::Less);
    /// assert_eq!(cmp(second, millis), Ordering::Greater);
    /// assert_eq!(cmp(millis, millis), Ordering::Equal);
    /// ```
    #[rune::function(keep, instance, protocol = CMP)]
    #[inline]
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(&self.inner, &rhs.inner)
    }

    /// Hash the duration.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::hash;
    ///
    /// use time::Duration;
    ///
    /// let second = Duration::SECOND;
    ///
    /// assert_eq!(hash(second), hash(second));
    /// ```
    #[rune::function(keep, instance, protocol = HASH)]
    fn hash(&self, hasher: &mut Hasher) {
        self.inner.hash(hasher);
    }

    /// Write a debug representation of the duration.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let second = Duration::SECOND;
    ///
    /// println!("{second:?}");
    /// ```
    #[rune::function(keep, instance, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{:?}", self.inner)
    }

    /// Clone the current duration.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let first = Duration::SECOND;
    /// let second = Duration::SECOND;
    /// second += Duration::SECOND;
    ///
    /// assert!(first < second);
    /// ```
    #[rune::function(keep, instance, protocol = CLONE)]
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}

mod const_duration {
    use rune::runtime::{ConstValue, RuntimeError, Value};
    use tokio::time::Duration;

    #[inline]
    pub(super) fn to_const_value(duration: Duration) -> Result<ConstValue, RuntimeError> {
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        rune::to_const_value((secs, nanos))
    }

    #[inline]
    pub(super) fn from_const_value(value: &ConstValue) -> Result<Duration, RuntimeError> {
        let (secs, nanos) = rune::from_const_value::<(u64, u32)>(value)?;
        Ok(Duration::new(secs, nanos))
    }

    #[inline]
    pub(super) fn from_value(value: Value) -> Result<Duration, RuntimeError> {
        let (secs, nanos) = rune::from_value::<(u64, u32)>(value)?;
        Ok(Duration::new(secs, nanos))
    }
}

/// Interval returned by [`interval`] and [`interval_at`].
///
/// This type allows you to wait on a sequence of instants with a certain
/// duration between each instant. Unlike calling [`sleep`] in a loop, this lets
/// you count the time spent between the calls to [`sleep`] as well.
#[derive(Debug, Any)]
#[rune(item = ::time)]
pub struct Interval {
    inner: tokio::time::Interval,
}

impl Interval {
    /// Completes when the next instant in the interval has been reached.
    ///
    /// # Cancel safety
    ///
    /// This method is cancellation safe. If `tick` is used as the branch in a `select` and
    /// another branch completes first, then no tick has been consumed.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let  interval = time::interval(Duration::from_millis(10));
    ///
    /// interval.tick().await;
    /// println!("approximately 0ms have elapsed. The first tick completes immediately.");
    /// interval.tick().await;
    /// interval.tick().await;
    ///
    /// println!("approximately 20ms have elapsed...");
    /// ```
    pub async fn tick(mut internal: Mut<Interval>) {
        internal.inner.tick().await;
    }

    /// Resets the interval to complete one period after the current time.
    ///
    /// This is equivalent to calling `reset_at(Instant::now() + period)`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let interval = time::interval(Duration::from_millis(100));
    /// interval.tick().await;
    ///
    /// time::sleep(Duration::from_millis(50)).await;
    /// interval.reset();
    ///
    /// interval.tick().await;
    /// interval.tick().await;
    ///
    /// println!("approximately 250ms have elapsed...");
    /// ```
    #[rune::function(instance, keep)]
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Resets the interval immediately.
    ///
    /// This is equivalent to calling `reset_at(Instant::now())`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let interval = time::interval(Duration::from_millis(100));
    /// interval.tick().await;
    ///
    /// time::sleep(Duration::from_millis(50)).await;
    /// interval.reset_immediately();
    ///
    /// interval.tick().await;
    /// interval.tick().await;
    ///
    /// println!("approximately 150ms have elapsed...");
    /// ```
    #[rune::function(instance, keep)]
    fn reset_immediately(&mut self) {
        self.inner.reset_immediately();
    }

    /// Resets the interval to complete one period after the current time.
    ///
    /// This is equivalent to calling `reset_at(Instant::now() + period)`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let interval = time::interval(Duration::from_millis(100));
    /// interval.tick().await;
    ///
    /// time::sleep(Duration::from_millis(50)).await;
    /// interval.reset();
    ///
    /// interval.tick().await;
    /// interval.tick().await;
    ///
    /// println!("approximately 250ms have elapsed...");
    /// ```
    #[rune::function(instance, keep)]
    fn reset_after(&mut self, after: Duration) {
        self.inner.reset_after(after.inner);
    }

    /// Resets the interval to complete one period after the current time.
    ///
    /// This is equivalent to calling `reset_at(Instant::now() + period)`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let interval = time::interval(Duration::from_millis(100));
    /// interval.tick().await;
    ///
    /// time::sleep(Duration::from_millis(50)).await;
    /// interval.reset();
    ///
    /// interval.tick().await;
    /// interval.tick().await;
    ///
    /// println!("approximately 250ms have elapsed...");
    /// ```
    #[rune::function(instance, keep)]
    fn reset_at(&mut self, deadline: Instant) {
        self.inner.reset_at(deadline.inner);
    }
}

/// A measurement of a monotonically nondecreasing clock.
/// Opaque and useful only with `Duration`.
///
/// Instants are always guaranteed to be no less than any previously measured
/// instant when created, and are often useful for tasks such as measuring
/// benchmarks or timing how long an operation takes.
///
/// Note, however, that instants are not guaranteed to be **steady**. In other
/// words, each tick of the underlying clock may not be the same length (e.g.
/// some seconds may be longer than others). An instant may jump forwards or
/// experience time dilation (slow down or speed up), but it will never go
/// backwards.
///
/// Instants are opaque types that can only be compared to one another. There is
/// no method to get "the number of seconds" from an instant. Instead, it only
/// allows measuring the duration between two instants (or comparing two
/// instants).
///
/// The size of an `Instant` struct may vary depending on the target operating
/// system.
#[derive(Debug, Any)]
#[rune(item = ::time)]
pub struct Instant {
    inner: tokio::time::Instant,
}

impl Instant {
    /// Returns an instant corresponding to `now`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let instant = Instant::now();
    /// ```
    #[rune::function(keep, path = Self::now)]
    pub fn now() -> Instant {
        Instant {
            inner: tokio::time::Instant::now(),
        }
    }

    /// Returns the amount of time elapsed from another instant to this one, or
    /// zero duration if that instant is later than this one.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::{Duration, Instant};
    ///
    /// let instant = Instant::now();
    ///
    /// let three_secs = Duration::from_secs(3);
    /// time::sleep(three_secs).await;
    ///
    /// let now = Instant::now();
    /// let duration_since = now.duration_since(instant);
    /// ```
    #[rune::function(instance, keep)]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration {
            inner: tokio::time::Instant::duration_since(&self.inner, earlier.inner),
        }
    }

    /// Returns the amount of time elapsed since this instant was created, or
    /// zero duration if that this instant is in the future.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::{Duration, Instant};
    ///
    /// let instant = Instant::now();
    ///
    /// let three_secs = Duration::from_secs(3);
    /// time::sleep(three_secs).await;
    ///
    /// let elapsed = instant.elapsed();
    /// ```
    #[rune::function(instance, keep)]
    pub fn elapsed(&self) -> Duration {
        Duration {
            inner: tokio::time::Instant::elapsed(&self.inner),
        }
    }

    /// Add a duration to this instant and return a new instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first + Duration::SECOND;
    ///
    /// assert!(first < second);
    /// ```
    #[rune::function(keep, instance, protocol = ADD)]
    #[inline]
    fn add(&self, rhs: &Duration) -> Result<Self, VmError> {
        let Some(inner) = self.inner.checked_add(rhs.inner) else {
            return Err(VmError::panic("overflow when adding duration to instant"));
        };

        Ok(Self { inner })
    }

    /// Add a duration to this instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::partial_eq;
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first.clone();
    /// second += Duration::SECOND;
    ///
    /// assert!(first < second);
    /// ```
    #[rune::function(keep, instance, protocol = ADD_ASSIGN)]
    #[inline]
    fn add_assign(&mut self, rhs: &Duration) -> Result<(), VmError> {
        let Some(inner) = self.inner.checked_add(rhs.inner) else {
            return Err(VmError::panic("overflow when adding duration to instant"));
        };

        self.inner = inner;
        Ok(())
    }

    /// Subtract a duration and return a new Instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first - Duration::SECOND;
    /// ```
    #[rune::function(keep, instance, protocol = SUB)]
    #[inline]
    fn sub(&self, duration: &Duration) -> VmResult<Self> {
        let Some(inner) = self.inner.checked_sub(duration.inner) else {
            vm_panic!("overflow when subtract duration from instant")
        };

        VmResult::Ok(Self { inner })
    }

    /// Subtract a duration from this instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// first -= Duration::SECOND;
    /// ```
    #[rune::function(keep, instance, protocol = SUB_ASSIGN)]
    #[inline]
    fn sub_assign(&mut self, duration: &Duration) -> VmResult<()> {
        let Some(inner) = self.inner.checked_sub(duration.inner) else {
            vm_panic!("overflow when subtract duration from instant")
        };

        self.inner = inner;
        VmResult::Ok(())
    }

    /// Subtract a instant and return a new Duration.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let duration = first - Instant::now();
    /// ```
    #[rune::function(keep, instance, protocol = SUB)]
    #[inline]
    fn sub_instant(&self, instant: &Instant) -> VmResult<Duration> {
        let Some(inner) = self.inner.checked_duration_since(instant.inner) else {
            vm_panic!("overflow when subtract instant")
        };

        VmResult::Ok(Duration::from_std(inner))
    }

    /// Subtract a instant from this instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::partial_eq;
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// first -= Instant::now();
    /// ```
    #[rune::function(keep, instance, protocol = SUB_ASSIGN)]
    #[inline]
    fn sub_instant_assign(&mut self, instant: &Instant) -> VmResult<()> {
        let Some(inner) = self.inner.checked_duration_since(instant.inner) else {
            vm_panic!("overflow when subtract instant")
        };

        self.inner -= inner;
        VmResult::Ok(())
    }

    /// Test two instants for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::partial_eq;
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first + Duration::SECOND;
    ///
    /// assert_eq!(partial_eq(first, first), true);
    /// assert_eq!(partial_eq(first, second), false);
    /// assert_eq!(partial_eq(second, first), false);
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_EQ)]
    #[inline]
    fn partial_eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Test two instants for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first + Duration::SECOND;
    ///
    /// assert_eq!(eq(first, first), true);
    /// assert_eq!(eq(first, second), false);
    /// assert_eq!(eq(second, first), false);
    /// ```
    #[rune::function(keep, instance, protocol = EQ)]
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        PartialEq::eq(&self.inner, &rhs.inner)
    }

    /// Perform a partial ordered comparison between two instants.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first + Duration::SECOND;
    ///
    /// assert!(first < second);
    /// assert!(second > first);
    /// assert!(first == first);
    /// ```
    ///
    /// Using explicit functions:
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::partial_cmp;
    ///
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first + Duration::SECOND;
    ///
    /// assert_eq!(partial_cmp(first, second), Some(Ordering::Less));
    /// assert_eq!(partial_cmp(second, first), Some(Ordering::Greater));
    /// assert_eq!(partial_cmp(first, first), Some(Ordering::Equal));
    /// ```
    #[rune::function(keep, instance, protocol = PARTIAL_CMP)]
    #[inline]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.inner, &rhs.inner)
    }

    /// Perform a totally ordered comparison between two instants.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::cmp::Ordering;
    /// use std::ops::cmp;
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first + Duration::SECOND;
    ///
    /// assert_eq!(cmp(first, second), Ordering::Less);
    /// assert_eq!(cmp(second, first), Ordering::Greater);
    /// assert_eq!(cmp(first, first), Ordering::Equal);
    /// ```
    #[rune::function(keep, instance, protocol = CMP)]
    #[inline]
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(&self.inner, &rhs.inner)
    }

    /// Hash the instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::hash;
    /// use time::{Duration, Instant};
    ///
    /// let now = Instant::now();
    ///
    /// assert_eq!(hash(now), hash(now));
    /// ```
    #[rune::function(keep, instance, protocol = HASH)]
    fn hash(&self, hasher: &mut Hasher) {
        self.inner.hash(hasher);
    }

    /// Write a debug representation of the instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Instant;
    ///
    /// let now = Instant::now();
    ///
    /// println!("{now:?}");
    /// ```
    #[rune::function(keep, instance, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{:?}", self.inner)
    }

    /// Clone the current instant.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::{Duration, Instant};
    ///
    /// let first = Instant::now();
    /// let second = first.clone();
    /// second += Duration::SECOND;
    ///
    /// assert!(first < second);
    /// ```
    #[rune::function(keep, instance, protocol = CLONE)]
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}
