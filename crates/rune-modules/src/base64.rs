use base64::prelude::*;
use rune::alloc::fmt::TryWrite;
use rune::alloc::{String, Vec};
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
    // estimate the max size
    let mut decoded_size = base64::decoded_len_estimate(inp.len());

    // try to allocate enough bytes
    let mut v: Vec<u8> = Vec::new();
    v.try_resize_with(decoded_size, || 0).vm?;
    let mut bytes = Bytes::from_vec(v);

    // decode
    let len = BASE64_STANDARD.decode_slice(inp, &mut bytes)?;

    // remove bytes which where to much
    while decoded_size > len {
        bytes.pop();
        decoded_size -= 1;
    }

    Ok(bytes)
}

/// Encode a data into a base64 String.
///
/// ```rune
/// assert_eq!(base64::encode(b"\xFF\xEC\x20\x55\0"), "/+wgVQA=");
/// ```
#[rune::function(vm_result)]
fn encode(bytes: &[u8]) -> String {
    let Some(encoded_size) = base64::encoded_len(bytes.len(), true) else {
        return VmResult::panic("Input bytes overflow usize");
    };

    let mut output_buf: Vec<u8> = Vec::new();
    output_buf
        .try_resize_with(encoded_size, Default::default)
        .vm?;

    // this should never panic
    if BASE64_STANDARD
        .encode_slice(bytes, &mut output_buf)
        .is_err()
    {
        VmResult::panic("Can't encode to base64 String").vm?;
    }

    // base64 should only return valid utf8 strings
    let Ok(string) = String::from_utf8(output_buf) else {
        return VmResult::panic("Can't crate valid utf8 string");
    };

    string
}

/// An error returned by methods in the `base64` module.
#[derive(Debug, rune::Any)]
#[rune(item = ::base64)]
#[allow(dead_code)]
pub struct DecodeError {
    inner: base64::DecodeSliceError,
}

impl From<base64::DecodeSliceError> for DecodeError {
    fn from(inner: base64::DecodeSliceError) -> Self {
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
