//! The native `fs` module for the [Rune Language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.0", features = ["fs"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::fs::module()?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! fn main() {
//!     let file = fs::read_to_string("file.txt").await?;
//!     println(`{file}`);
//! }
//! ```

use std::io;
use tokio::fs;

/// Construct the `fs` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["fs"]);
    module.async_function(&["read_to_string"], read_to_string)?;
    Ok(module)
}

async fn read_to_string(path: &str) -> io::Result<String> {
    fs::read_to_string(path).await
}
