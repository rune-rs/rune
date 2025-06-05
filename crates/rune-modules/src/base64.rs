use base64::prelude::*;
use rune::alloc::fmt::TryWrite;
use rune::alloc::{self, String, Vec};
use rune::runtime::{Bytes, Formatter, VmError};
use rune::{nested_try, ContextError, Module};

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
#[rune::module(::base64)]
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
#[rune::function]
fn decode(inp: &str) -> alloc::Result<Result<Bytes, DecodeError>> {
    // estimate the max size
    let decoded_size = base64::decoded_len_estimate(inp.len());

    // try to allocate enough bytes
    let mut v = Vec::new();

    v.try_resize(decoded_size, 0)?;

    // decode
    let len = nested_try!(BASE64_STANDARD.decode_slice(inp, &mut v));

    v.truncate(len);
    Ok(Ok(Bytes::from_vec(v)))
}

/// Encode a data into a base64 String.
///
/// # Examples
///
/// ```rune
/// assert_eq!(base64::encode(b"\xFF\xEC\x20\x55\0"), "/+wgVQA=");
/// ```
#[rune::function]
fn encode(bytes: &[u8]) -> Result<String, VmError> {
    let Some(encoded_size) = base64::encoded_len(bytes.len(), true) else {
        return Err(VmError::panic("encoded input length overflows usize"));
    };

    let mut buf = Vec::new();
    buf.try_resize(encoded_size, 0)?;

    // this should never panic
    if let Err(e) = BASE64_STANDARD.encode_slice(bytes, &mut buf) {
        return Err(VmError::panic(e));
    }

    // base64 should only return valid utf8 strings
    let string = String::from_utf8(buf).map_err(VmError::panic)?;

    Ok(string)
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
    fn display_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{}", self.inner)
    }
}
