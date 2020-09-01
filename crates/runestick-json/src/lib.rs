//! The json package, providing access to functions to serialize and deserialize
//! json.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! runestick = "0.2"
//! runestick-json = "0.2"
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_packages()?;
//! context.install(&runestick_json::module()?)?;
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

use runestick::{Bytes, ContextError, Module, Value};

fn from_bytes(bytes: &[u8]) -> runestick::Result<Value> {
    Ok(serde_json::from_slice(&bytes)?)
}

/// Get value from json string.
fn from_string(string: &str) -> runestick::Result<Value> {
    Ok(serde_json::from_str(string)?)
}

/// Convert any value to a json string.
fn to_string(value: Value) -> runestick::Result<String> {
    Ok(serde_json::to_string(&value)?)
}

/// Convert any value to a json string.
fn to_bytes(value: Value) -> runestick::Result<Bytes> {
    let bytes = serde_json::to_vec(&value)?;
    Ok(Bytes::from_vec(bytes))
}

/// Get the module for the bytes package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["json"]);
    module.function(&["from_bytes"], from_bytes)?;
    module.function(&["from_string"], from_string)?;
    module.function(&["to_string"], to_string)?;
    module.function(&["to_bytes"], to_bytes)?;
    Ok(module)
}
