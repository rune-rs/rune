//! The native `time` module for the [Rune Language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.2", features = ["time"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::time::module()?)?;
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
//!     time::delay_for(time::Duration::from_secs(10)).await;
//!     println("Message after 10 seconds!");
//! }
//! ```

use runestick::{ContextError, Module};

/// Construct the `time` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["time"]);
    module.function(&["Duration", "from_secs"], Duration::from_secs)?;
    module.async_function(&["delay_for"], delay_for)?;
    Ok(module)
}

#[derive(Debug, Clone, Copy)]
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
async fn delay_for(duration: &Duration) {
    tokio::time::delay_for(duration.inner).await;
}

runestick::decl_external!(Duration);
