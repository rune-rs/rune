//! The json package, providing access to functions to serialize and deserialize
//! json.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! stk = "0.2"
//! stk-json = "0.2"
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> stk::Result<()> {
//! let mut context = stk::Context::with_default_packages()?;
//! context.install(stk_json::module()?)?;
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

use stk::packages::bytes::Bytes;
use stk::{ContextError, Module, ValuePtr};

fn from_bytes(bytes: &[u8]) -> stk::Result<ValuePtr> {
    Ok(serde_json::from_slice(&bytes)?)
}

/// Get value from json string.
fn from_string(string: &str) -> stk::Result<ValuePtr> {
    Ok(serde_json::from_str(string)?)
}

/// Convert any value to a json string.
fn to_string(value: ValuePtr) -> stk::Result<String> {
    Ok(serde_json::to_string(&value)?)
}

/// Convert any value to a json string.
fn to_bytes(value: ValuePtr) -> stk::Result<Bytes> {
    let bytes = serde_json::to_vec(&value)?;
    Ok(Bytes::from_bytes(bytes))
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
