//! The native `json` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.3", features = ["json"] }
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

/// Construct the `json` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("json")?;
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
#[rune::function]
fn from_bytes(bytes: &[u8]) -> Result<Value, Error> {
    Ok(serde_json::from_slice(bytes)?)
}

/// Convert a JSON string into a rune value.
#[rune::function]
fn from_string(string: &str) -> Result<Value, Error> {
    Ok(serde_json::from_str(string)?)
}

/// Convert any value to a json string.
#[rune::function(vm_result)]
fn to_string(value: Value) -> Result<String, Error> {
    Ok(String::try_from(serde_json::to_string(&value)?).vm?)
}

/// Convert any value to json bytes.
#[rune::function(vm_result)]
fn to_bytes(value: Value) -> Result<Bytes, Error> {
    Ok(Bytes::from_vec(Vec::try_from(serde_json::to_vec(&value)?).vm?))
}
