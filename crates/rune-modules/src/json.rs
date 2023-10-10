//! The native `json` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.13.1", features = ["json"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::json::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
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

use rune::{ContextError, Module, vm_write, Any};
use rune::runtime::{Bytes, Value, Formatter};
use rune::alloc::{Vec, String};
use rune::alloc::fmt::TryWrite;

#[rune::module(::json)]
/// Module for processing JSON.
///
/// # Examples
///
/// ```rune
/// let object = #{"number": 42, "string": "Hello World"};
/// let object = json::from_string(json::to_string(object)?)?;
/// assert_eq!(object, #{"number": 42, "string": "Hello World"});
/// ```
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Error>()?;
    module.function_meta(Error::display)?;
    module.function_meta(Error::debug)?;
    module.function_meta(from_bytes)?;
    module.function_meta(from_string)?;
    module.function_meta(to_string)?;
    module.function_meta(to_bytes)?;
    Ok(module)
}

#[derive(Any)]
#[rune(item = ::json)]
/// Error type raised during JSON serialization.
struct Error {
    error: serde_json::Error,
}

impl Error {
    #[rune::function(vm_result, protocol = STRING_DISPLAY)]
    pub(crate) fn display(&self, f: &mut Formatter) {
        vm_write!(f, "{}", self.error);
    }

    #[rune::function(vm_result, protocol = STRING_DEBUG)]
    pub(crate) fn debug(&self, f: &mut Formatter) {
        vm_write!(f, "{:?}", self.error);
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self { error }
    }
}

/// Convert JSON bytes into a rune value.
/// 
/// # Examples
/// 
/// ```rune
/// let object = json::from_bytes(b"{\"number\": 42, \"string\": \"Hello World\"}")?;
/// assert_eq!(object, #{"number": 42, "string": "Hello World"});
/// ```
#[rune::function]
fn from_bytes(bytes: &[u8]) -> Result<Value, Error> {
    Ok(serde_json::from_slice(bytes)?)
}

/// Convert a JSON string into a rune value.
/// 
/// # Examples
/// 
/// ```rune
/// let object = json::from_string("{\"number\": 42, \"string\": \"Hello World\"}")?;
/// assert_eq!(object, #{"number": 42, "string": "Hello World"});
/// ```
#[rune::function]
fn from_string(string: &str) -> Result<Value, Error> {
    Ok(serde_json::from_str(string)?)
}

/// Convert any value to a json string.
/// 
/// # Examples
/// 
/// ```rune
/// let object = #{"number": 42, "string": "Hello World"};
/// let object = json::from_string(json::to_string(object)?)?;
/// assert_eq!(object, #{"number": 42, "string": "Hello World"});
/// ```
#[rune::function(vm_result)]
fn to_string(value: Value) -> Result<String, Error> {
    Ok(String::try_from(serde_json::to_string(&value)?).vm?)
}

/// Convert any value to json bytes.
/// 
/// # Examples
/// 
/// ```rune
/// let object = #{"number": 42, "string": "Hello World"};
/// let object = json::from_bytes(json::to_bytes(object)?)?;
/// assert_eq!(object, #{"number": 42, "string": "Hello World"});
/// ```
#[rune::function(vm_result)]
fn to_bytes(value: Value) -> Result<Bytes, Error> {
    Ok(Bytes::from_vec(Vec::try_from(serde_json::to_vec(&value)?).vm?))
}
