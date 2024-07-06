use base64::prelude::*;
use rune::{ContextError, Module};

/// Construct the `base64` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("base64")?;
    module.item_mut().docs([
        "Base64 encoding and decoding",
        "",
        "This module provides functions to encode & decode base64 data.",
    ])?;

    module.function_meta(decode)?;
    module.function_meta(encode)?;
    Ok(module)
}

/// Decode a base64 String into data
///
/// ```rune
/// //assert_eq!(base64::decode("+uwgVQA=")?, b"\xFA\xEC\x20\x55\0");
/// assert!(base64::decode("+uwgVQA=").is_ok());
/// ```
#[rune::function]
fn decode(inp: &str) -> Result<Vec<u8>, String> {
    BASE64_STANDARD.decode(inp).map_err(|e| e.to_string())
}

/// Encode a data into a base64 String.
///
/// ```rune
/// assert_eq!(base64::encode(b"\xFF\xEC\x20\x55\0"), "/+wgVQA=");
/// ```
#[rune::function]
fn encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}
