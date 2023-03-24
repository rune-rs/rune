//! The native `toml` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.2", features = ["toml"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::toml::module(true)?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use toml;
//!
//! fn main() {
//!     let data = toml::from_string("[hello]\nworld = 42");
//!     dbg(data);
//! }
//! ```

use rune::{ContextError, Module};
use rune::runtime::{Bytes, Value};

/// Construct the `toml` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("toml");
    module.function(["from_bytes"], from_bytes)?;
    module.function(["from_string"], from_string)?;
    module.function(["to_string"], to_string)?;
    module.function(["to_bytes"], to_bytes)?;
    Ok(module)
}

fn from_bytes(bytes: &[u8]) -> rune::Result<Value> {
    Ok(toml::from_slice(bytes)?)
}

/// Get value from toml string.
fn from_string(string: &str) -> rune::Result<Value> {
    Ok(toml::from_str(string)?)
}

/// Convert any value to a toml string.
fn to_string(value: Value) -> rune::Result<String> {
    Ok(toml::to_string(&value)?)
}

/// Convert any value to toml bytes.
fn to_bytes(value: Value) -> rune::Result<Bytes> {
    let bytes = toml::to_vec(&value)?;
    Ok(Bytes::from_vec(bytes))
}
