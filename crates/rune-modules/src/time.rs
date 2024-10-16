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

use rune::{
    docstring,
    runtime::{Mut, VmResult},
    vm_panic, Any, ContextError, Module,
};

const NANOS_PER_SEC: u32 = 1_000_000_000;

/// Construct the `time` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("time")?;

    module.ty::<Duration>()?;
    module.ty::<Interval>()?;
    module.ty::<Instant>()?;

    module.function_meta(Duration::new__meta)?;
    module.function_meta(Duration::from_secs__meta)?;
    module.function_meta(Duration::from_millis__meta)?;
    module.function_meta(Duration::from_micros__meta)?;
    module.function_meta(Duration::from_nanos__meta)?;
    module.function_meta(Duration::as_secs_f64__meta)?;
    module.function_meta(Duration::from_secs_f64__meta)?;

    /* TODO: Make Duration a ConstValue
    module
        .constant("SECOND", Duration::from_secs(1))
        .build_associated::<Duration>()?
        .docs(docstring! {
            /// The duration of one second.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use time::Duration;
            ///
            /// let duration = Duration::SECOND;
            /// ```
        })?;

    module
        .constant("MILLISECOND", Duration::from_millis(1))
        .build_associated::<Duration>()?
        .docs(docstring! {
            /// The duration of one millisecond.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use time::Duration;
            ///
            /// let duration = Duration::MILLISECOND;
            /// ```
        })?;

    module
        .constant("MICROSECOND", Duration::from_micros(1))
        .build_associated::<Duration>()?
        .docs(docstring! {
            /// The duration of one microsecond.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use time::Duration;
            ///
            /// let duration = Duration::MICROSECOND;
            /// ```
        })?;

    module
        .constant("NANOSECOND", Duration::from_nanos(1))
        .build_associated::<Duration>()?
        .docs(docstring! {
            /// The duration of one nanosecond.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use time::Duration;
            ///
            /// let duration = Duration::NANOSECOND;
            /// ```
        })?;

    module
        .constant("ZERO", Duration::from_nanos(0))
        .build_associated::<Duration>()?
        .docs(docstring! {
            /// A duration of zero time.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use time::Duration;
            ///
            /// let duration = Duration::ZERO;
            /// ```
        })?;

    module
        .constant("MAX", Duration::new(u64::MAX, NANOS_PER_SEC - 1))
        .build_associated::<Duration>()?
        .docs(docstring! {
            /// The maximum duration.
            ///
            /// # Examples
            ///
            /// ```rune,no_run
            /// use time::Duration;
            ///
            /// let duration = Duration::MAX;
            /// ```
        })?;
    */

    module
        .function("tick", Interval::tick)
        .build_associated::<Interval>()?;
    module.function_meta(Interval::reset__meta)?;
    module.function_meta(Interval::reset_immediately__meta)?;
    module.function_meta(Interval::reset_after__meta)?;
    module.function_meta(Interval::reset_at__meta)?;

    module.function_meta(Instant::now__meta)?;
    module.function_meta(Instant::duration_since__meta)?;
    module.function_meta(Instant::elapsed__meta)?;

    module.function_meta(sleep)?;
    module.function_meta(interval)?;
    module.function_meta(interval_at)?;

    Ok(module)
}

/// Waits until duration has elapsed.
///
/// # Examples
///
/// ```rune,no_run
/// use time::Duration;
///
/// let duration = Duration::from_secs(10);
/// time::sleep(d).await;
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
/// ```rune,no_run
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
#[derive(Debug, Clone, Copy, Any)]
#[rune(item = ::time)]
pub struct Duration {
    inner: tokio::time::Duration,
}

impl Duration {
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
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let five_seconds = Duration::new(5, 0);
    /// ```
    #[rune::function(keep, path = Self::new)]
    pub fn new(secs: u64, nanos: u32) -> VmResult<Self> {
        if nanos >= NANOS_PER_SEC {
            if secs.checked_add((nanos / NANOS_PER_SEC) as u64).is_none() {
                vm_panic!("overflow in Duration::new");
            }
        }

        VmResult::Ok(Self {
            inner: tokio::time::Duration::new(secs, nanos),
        })
    }

    /// Creates a new `Duration` from the specified number of whole seconds.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
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
    /// ```rune,no_run
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
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let duration = Duration::from_micros(1_000_002);
    /// ```
    #[rune::function(keep, path = Self::from_micros)]
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
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let duration = Duration::from_nanos(1_000_000_123);
    /// ```
    #[rune::function(keep, path = Self::from_nanos)]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_nanos(nanos),
        }
    }

    /// Returns the number of seconds contained by this `Duration` as `f64`.
    ///
    /// The returned value does include the fractional (nanosecond) part of the duration.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let duration = Duration::from_secs(60).as_secs_f64();
    /// ```
    #[rune::function(keep, path = Self::as_secs_f64)]
    pub const fn as_secs_f64(&self) -> f64 {
        self.inner.as_secs_f64()
    }

    /// Creates a new `Duration` from the specified number of seconds represented
    /// as `f64`.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::Duration;
    ///
    /// let duration = Duration::from_secs_f64(0.0);
    /// ```
    #[rune::function(keep, path = Self::from_secs_f64)]
    pub fn from_secs_f64(secs: f64) -> VmResult<Self> {
        match tokio::time::Duration::try_from_secs_f64(secs) {
            Ok(duration) => VmResult::Ok(Self { inner: duration }),
            Err(e) => vm_panic!(e),
        }
    }
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
    /// use time::Duration;
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
    /// use time::Duration;
    ///
    /// let tokio_duration = tokio::time::Duration::from_secs(5);
    /// let duration = Duration::from_tokio(tokio_duration);
    /// ```
    pub fn from_tokio(duration: tokio::time::Duration) -> Self {
        Self { inner: duration }
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
    /// ```rune,no_run
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
    /// use time::{Instant, Duration};
    ///
    /// let instant = Instant::now();
    ///
    /// let three_secs = Duration::from_secs(3);
    /// sleep(three_secs).await;
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

    /// Returns the amount of time elapsed since this instant was created,
    /// or zero duration if that this instant is in the future.
    ///
    /// # Examples
    ///
    /// ```rune,no_run
    /// use time::{Duration, Instant};
    ///
    /// let instant = Instant::now();
    ///
    /// let three_secs = Duration::from_secs(3);
    /// sleep(three_secs).await;
    ///
    /// let elapsed = instant.elapsed();
    /// ```
    #[rune::function(instance, keep)]
    pub fn elapsed(&self) -> Duration {
        Duration {
            inner: tokio::time::Instant::elapsed(&self.inner),
        }
    }
}
