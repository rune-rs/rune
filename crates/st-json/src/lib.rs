//! The json package, providing access to functions to serialize and deserialize
//! json.

use st::{Module, RegisterError, ValuePtr};

fn from_bytes(bytes: &[u8]) -> st::Result<ValuePtr> {
    Ok(serde_json::from_slice(&bytes)?)
}

fn from_string(string: &str) -> st::Result<ValuePtr> {
    Ok(serde_json::from_str(string)?)
}

/// Get the module for the bytes package.
pub fn module() -> Result<Module, RegisterError> {
    let mut module = Module::new(&["json"]);
    module.global_fallible_fn("from_bytes", from_bytes)?;
    module.global_fallible_fn("from_string", from_string)?;
    Ok(module)
}
