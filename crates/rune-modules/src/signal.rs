//! The native `signal` module for the [Rune Language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.15", features = ["signal"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::signal::module()?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! fn main() {
//!     signal::ctrl_c().await?;
//!     println("Exiting...");
//! }
//! ```

use tokio::signal;

/// Construct the `signal` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["signal"]);
    module.async_function(&["ctrl_c"], signal::ctrl_c)?;
    Ok(module)
}
