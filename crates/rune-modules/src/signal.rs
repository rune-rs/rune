//! The native `signal` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.1", features = ["signal"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::signal::module(true)?)?;
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
use rune::{Module, ContextError};

/// Construct the `signal` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("signal");
    module.async_function(["ctrl_c"], signal::ctrl_c)?;
    Ok(module)
}
