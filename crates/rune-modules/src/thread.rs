//! The native `thread` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.14.0", features = ["thread"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::thread::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use time::Duration;
//!
//! fn main() {
//!     thread::sleep(Duration::from_seconds(3));
//! }
//! ```

use crate::time::Duration;
use rune::{ContextError, Module};
use std::thread;

/// Construct the `thread` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("thread")?;
    module.function("sleep", sleep).build()?;
    Ok(module)
}

/// Puts the current thread to sleep for at least the specified amount of time.
fn sleep(duration: &Duration) {
    thread::sleep(duration.into_std())
}
