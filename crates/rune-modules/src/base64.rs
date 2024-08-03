use base64::prelude::*;
use rune::alloc::fmt::TryWrite;
use rune::alloc::String;
use rune::runtime::Bytes;
use rune::{
    runtime::{Formatter, VmResult},
    ContextError, Module,
};

/// Construct the `base64` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("base64")?;
    module.item_mut().docs([
        "Base64 encoding and decoding",
        "",
        "This module provides functions to encode & decode base64 data.",
    ])?;

    module.ty::<DecodeError>()?;

    module.function_meta(decode)?;
    module.function_meta(encode)?;
    Ok(module)
}

/// Decode a base64 String into data
///
/// ```rune
/// assert_eq!(base64::decode("+uwgVQA=")?, b"\xFA\xEC\x20\x55\0");
/// ```
#[rune::function(vm_result)]
fn decode(inp: &str) -> Result<Bytes, DecodeError> {
    Ok(Bytes::try_from(BASE64_STANDARD.decode(inp)?).vm?)
}

/// Encode a data into a base64 String.
///
/// ```rune
/// assert_eq!(base64::encode(b"\xFF\xEC\x20\x55\0"), "/+wgVQA=");
/// ```
#[rune::function(vm_result)]
fn encode(bytes: &[u8]) -> String {
    String::try_from(BASE64_STANDARD.encode(bytes)).vm?
}

/// An error returned by methods in the `base64` module.
#[derive(Debug, rune::Any)]
#[rune(item = ::base64)]
#[allow(dead_code)]
pub struct DecodeError {
    inner: base64::DecodeError,
}

impl From<base64::DecodeError> for DecodeError {
    fn from(inner: base64::DecodeError) -> Self {
        Self { inner }
    }
}

impl DecodeError {
    #[rune::function(instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner);
        VmResult::Ok(())
    }
}
