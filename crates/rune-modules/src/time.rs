//! The native `time` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.13.1", features = ["time"] }
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

/// Construct the `time` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("time")?;
    module.ty::<Duration>()?;
    module.function_meta(Duration::from_secs__meta)?;
    module.function_meta(sleep)?;
    Ok(module)
}

#[derive(Debug, Clone, Copy, Any)]
#[rune(item = ::time)]
struct Duration {
    inner: tokio::time::Duration,
}

impl Duration {
    /// Construct a duration from the given number of seconds.
    /// 
    /// # Examples
    /// 
    /// ```rune
    /// use time::Duration;
    ///
    /// let d = Duration::from_secs(10);
    /// ```
    #[rune::function(keep, path = Self::from_secs)]
    fn from_secs(secs: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_secs(secs),
        }
    }
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
