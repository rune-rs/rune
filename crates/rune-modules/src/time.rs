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

use rune::{Any, ContextError, Module};

const NANOS_PER_SEC: u32 = 1_000_000_000;
const NANOS_PER_MILLI: u32 = 1_000_000;
const NANOS_PER_MICRO: u32 = 1_000;
const MILLIS_PER_SEC: u64 = 1_000;
const MICROS_PER_SEC: u64 = 1_000_000;

/// Construct the `time` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("time")?;

    module_duration(&mut module)?;
    module_internal(&mut module)?;
    module_instant(&mut module)?;

    module.function_meta(sleep)?;
    module.function_meta(interval)?;
    // module.function_meta(timeout)?;

    Ok(module)
}

/// Sleep for the given [`Duration`].
///
/// # Examples
///
/// ```rune,no_run
/// use time::Duration;
///
/// let d = Duration::from_secs(10);
/// time::sleep(d).await;
/// println!("Surprise!");
/// ```
#[rune::function]
async fn sleep(duration: Duration) {
    tokio::time::sleep(duration.inner).await;
}

#[rune::function]
async fn interval(duration: Duration) -> Interval {
    Interval {
        inner: tokio::time::interval(duration.inner),
    }
}

// #[rune::function]
// async fn timeout(duration: Duration, future: F) -> Timeout<F>
// where
//     F: Future + rune::compile::Named + CoreTypeOf + MaybeTypeOf + 'static,
// {
//     Timeout {
//         inner: tokio::time::timeout(duration.inner, future).await,
//     }
// }

fn module_duration(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<Duration>()?;

    module.function_meta(Duration::new__meta)?;
    module.function_meta(Duration::from_secs__meta)?;
    module.function_meta(Duration::from_millis__meta)?;
    module.function_meta(Duration::from_micros__meta)?;
    module.function_meta(Duration::from_nanos__meta)?;
    module.function_meta(Duration::as_secs_f64__meta)?;
    module.function_meta(Duration::from_secs_f64__meta)?;

    module
        .constant("SECOND", Duration::from_secs(1))
        .build_associated::<Duration>()?
        .docs(["The duration of one second."])?;

    module
        .constant("MILLISECOND", Duration::from_millis(1))
        .build_associated::<Duration>()?
        .docs(["The duration of one millisecond."])?;

    module
        .constant("MICROSECOND", Duration::from_micros(1))
        .build_associated::<Duration>()?
        .docs(["The duration of one microsecond."])?;

    module
        .constant("NANOSECOND", Duration::from_nanos(1))
        .build_associated::<Duration>()?
        .docs(["The duration of one nanosecond."])?;

    module
        .constant("ZERO", Duration::from_nanos(0))
        .build_associated::<Duration>()?
        .docs(["A duration of zero time."])?;

    module
        .constant("MAX", Duration::new(u64::MAX, NANOS_PER_SEC - 1))
        .build_associated::<Duration>()?
        .docs(["A duration of zero time."])?;

    Ok(())
}

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
    /// # Panics
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
    pub fn new(secs: u64, nanos: u32) -> Self {
        Self {
            inner: tokio::time::Duration::new(secs, nanos),
        }
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
    pub fn from_secs(secs: u64) -> Self {
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
    pub fn from_millis(millis: u64) -> Self {
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
    pub fn from_micros(micros: u64) -> Self {
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
    pub fn from_nanos(nanos: u64) -> Self {
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
    /// ```rune
    /// use time::Duration;
    ///
    /// let d = Duration::from_secs(60).as_secs_f64();
    /// ```
    #[rune::function(keep, path = Self::as_secs_f64)]
    pub fn as_secs_f64(&self) -> f64 {
        self.inner.as_secs_f64()
    }

    /// Creates a new `Duration` from the specified number of seconds represented
    /// as `f64`.
    ///
    /// # Panics
    /// This constructor will panic if `secs` is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use time::Duration;
    ///
    /// let res = Duration::from_secs_f64(0.0);
    /// ```
    #[rune::function(keep, path = Self::from_secs_f64)]
    pub fn from_secs_f64(secs: f64) -> Self {
        Self {
            inner: tokio::time::Duration::from_secs_f64(secs),
        }
    }
}

fn module_internal(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<Interval>()?;

    // ERROR: convert trait
    module
        .function("tick", Interval::tick)
        .build_associated::<Interval>()?;

    module.function_meta(Interval::reset__meta)?;
    module.function_meta(Interval::reset_immediately__meta)?;
    module.function_meta(Interval::reset_after__meta)?;
    module.function_meta(Interval::reset_at__meta)?;

    Ok(())
}

#[derive(Debug, Any)]
#[rune(item = ::time)]
pub struct Interval {
    inner: tokio::time::Interval,
}

impl Interval {
    pub async fn tick(&mut self) {
        self.inner.tick().await;
    }

    #[rune::function(instance, keep)]
    fn reset(&mut self) {
        self.inner.reset();
    }

    #[rune::function(instance, keep)]
    fn reset_immediately(&mut self) {
        self.inner.reset_immediately();
    }

    #[rune::function(instance, keep)]
    fn reset_after(&mut self, after: Duration) {
        self.inner.reset_after(after.inner);
    }

    #[rune::function(instance, keep)]
    fn reset_at(&mut self, deadline: Instant) {
        self.inner.reset_at(deadline.inner);
    }
}

fn module_instant(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<Instant>()?;

    module.function_meta(Instant::now__meta)?;
    module.function_meta(Instant::duration_since__meta)?;
    module.function_meta(Instant::elapsed__meta)?;

    Ok(())
}

#[derive(Debug, Any)]
#[rune(item = ::time)]
pub struct Instant {
    inner: tokio::time::Instant,
}

impl Instant {
    #[rune::function(keep, path = Self::now)]
    pub fn now() -> Instant {
        Instant {
            inner: tokio::time::Instant::now(),
        }
    }

    #[rune::function(instance, keep)]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration {
            inner: tokio::time::Instant::duration_since(&self.inner, earlier.inner),
        }
    }

    #[rune::function(instance, keep)]
    pub fn elapsed(&self) -> Duration {
        Duration {
            inner: tokio::time::Instant::elapsed(&self.inner),
        }
    }
}

// #[derive(Debug, Any)]
// #[rune(item = ::time)]
// pub struct Timeout<T>
// where
//     T: Future + rune::compile::Named + CoreTypeOf + MaybeTypeOf + 'static,
// {
//     inner: tokio::time::Timeout<T>,
// }
