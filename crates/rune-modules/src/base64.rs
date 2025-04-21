use base64::prelude::*;
use rune::alloc::fmt::TryWrite;
use rune::alloc::{String, Vec};
use rune::runtime::Bytes;
use rune::runtime::{Formatter, VmResult};
use rune::{vm_panic, ContextError, Module};

#[rune::module(::base64)]
/// Correct and fast [base64] encoding based on the [`base64`] crate.
///
/// [base64]: https://developer.mozilla.org/en-US/docs/Glossary/Base64
/// [`base64`]: https://docs.rs/base64
///
/// # Examples
///
/// ```rune
/// let encoded = base64::encode(b"\xFF\xEC\x20\x55\0");
/// assert_eq!(base64::decode(encoded), Ok(b"\xFF\xEC\x20\x55\0"));
/// ```
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;

    m.ty::<DecodeError>()?;

    m.function_meta(decode)?;
    m.function_meta(encode)?;
    Ok(m)
}

/// Decode a base64 String into data
///
/// # Examples
///
/// ```rune
/// assert_eq!(base64::decode("+uwgVQA=")?, b"\xFA\xEC\x20\x55\0");
/// ```
#[rune::function(vm_result)]
fn decode(inp: &str) -> Result<Bytes, DecodeError> {
    // estimate the max size
    let decoded_size = base64::decoded_len_estimate(inp.len());

    // try to allocate enough bytes
    let mut v = Vec::new();
    v.try_resize(decoded_size, 0).vm?;

    // decode
    let len = BASE64_STANDARD.decode_slice(inp, &mut v)?;
    v.truncate(len);
    Ok(Bytes::from_vec(v))
}

/// Encode a data into a base64 String.
///
/// # Examples
///
/// ```rune
/// assert_eq!(base64::encode(b"\xFF\xEC\x20\x55\0"), "/+wgVQA=");
/// ```
#[rune::function(vm_result)]
fn encode(bytes: &[u8]) -> String {
    let Some(encoded_size) = base64::encoded_len(bytes.len(), true) else {
        vm_panic!("encoded input length overflows usize");
    };

    let mut buf = Vec::new();
    buf.try_resize(encoded_size, 0).vm?;

    // this should never panic
    if let Err(e) = BASE64_STANDARD.encode_slice(bytes, &mut buf) {
        vm_panic!(e);
    }

    // base64 should only return valid utf8 strings
    let string = match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => vm_panic!(e),
    };

    string
}

/// Errors that can occur while decoding.
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
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner)
    }
}
