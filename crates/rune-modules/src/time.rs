//! The native `time` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.10.1", features = ["time"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::time::module(true)?)?;
//! # Ok(())
//! # }
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
    let mut module = Module::with_crate("time");
    module.function(&["Duration", "from_secs"], Duration::from_secs)?;
    module.async_function(&["sleep"], sleep)?;
    Ok(module)
}

#[derive(Debug, Clone, Copy, Any)]
struct Duration {
    inner: tokio::time::Duration,
}

impl Duration {
    /// Construct a duration from seconds.
    fn from_secs(secs: u64) -> Self {
        Self {
            inner: tokio::time::Duration::from_secs(secs),
        }
    }
}

/// Convert any value to a json string.
async fn sleep(duration: &Duration) {
    tokio::time::sleep(duration.inner).await;
}
