//! The json package, providing access to functions to serialize and deserialize
//! json.

use st::packages::bytes::Bytes;
use st::{ContextError, Module, ValuePtr};

fn from_bytes(bytes: &[u8]) -> st::Result<ValuePtr> {
    Ok(serde_json::from_slice(&bytes)?)
}

/// Get value from json string.
fn from_string(string: &str) -> st::Result<ValuePtr> {
    Ok(serde_json::from_str(string)?)
}

/// Convert any value to a json string.
fn to_string(value: ValuePtr) -> st::Result<String> {
    Ok(serde_json::to_string(&value)?)
}

/// Convert any value to a json string.
fn to_bytes(value: ValuePtr) -> st::Result<Bytes> {
    let bytes = serde_json::to_vec(&value)?;
    Ok(Bytes::from_bytes(bytes))
}

/// Get the module for the bytes package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["json"]);
    module.fallible_free_fn("from_bytes", from_bytes)?;
    module.fallible_free_fn("from_string", from_string)?;
    module.fallible_free_fn("to_string", to_string)?;
    module.fallible_free_fn("to_bytes", to_bytes)?;
    Ok(module)
}
