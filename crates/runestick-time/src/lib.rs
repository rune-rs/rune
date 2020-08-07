//! The runestick time package.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! runestick = "0.2"
//! runestick-timer = "0.2"
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_packages()?;
//! context.install(runestick_time::module()?)?;
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
//! }
//! ```

use runestick::{ContextError, Module};

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
async fn delay_for(duration: Duration) {
    tokio::time::delay_for(duration.inner).await;
}

runestick::decl_external!(Duration);

/// Get the module for the bytes package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["time"]);
    module.function(&["Duration", "from_secs"], Duration::from_secs)?;
    module.async_function(&["delay_for"], delay_for)?;
    Ok(module)
}
