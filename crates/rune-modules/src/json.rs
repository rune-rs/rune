//! The native `json` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.2", features = ["json"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::json::module(true)?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use json;
//!
//! fn main() {
//!     let data = json::from_string("{\"key\": 42}");
//!     dbg(data);
//! }
//! ```

use rune::{ContextError, Module};
use rune::runtime::{Bytes, Value};

/// Construct the `json` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("json");
    module.function(["from_bytes"], from_bytes)?;
    module.function(["from_string"], from_string)?;
    module.function(["to_string"], to_string)?;
    module.function(["to_bytes"], to_bytes)?;
    Ok(module)
}

fn from_bytes(bytes: &[u8]) -> rune::Result<Value> {
    Ok(serde_json::from_slice(bytes)?)
}

/// Get value from json string.
fn from_string(string: &str) -> rune::Result<Value> {
    Ok(serde_json::from_str(string)?)
}

/// Convert any value to a json string.
fn to_string(value: Value) -> rune::Result<String> {
    Ok(serde_json::to_string(&value)?)
}

/// Convert any value to json bytes.
fn to_bytes(value: Value) -> rune::Result<Bytes> {
    let bytes = serde_json::to_vec(&value)?;
    Ok(Bytes::from_vec(bytes))
}
