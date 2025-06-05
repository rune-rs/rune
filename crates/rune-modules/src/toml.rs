//! The native `toml` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.14.0", features = ["toml"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::toml::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! use toml;
//!
//! fn main() {
//!     let data = toml::from_string("[hello]\nworld = 42");
//!     dbg(data);
//! }
//! ```

use rune::alloc::{self, String};
use rune::runtime::{Bytes, Value};
use rune::{nested_try, ContextError, Module, VmError};

/// Construct the `toml` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("toml")?;
    module.function_meta(from_bytes)?;
    module.function_meta(from_string)?;
    module.function_meta(to_string)?;
    module.function_meta(to_bytes)?;
    Ok(module)
}

pub mod de {
    //! Deserializer types for the toml module.

    use rune::alloc;
    use rune::alloc::fmt::TryWrite;
    use rune::runtime::Formatter;
    use rune::{Any, ContextError, Module};

    pub fn module(_stdio: bool) -> Result<Module, ContextError> {
        let mut module = Module::with_crate_item("toml", ["de"])?;
        module.ty::<Error>()?;
        module.function_meta(Error::display)?;
        module.function_meta(Error::debug)?;
        Ok(module)
    }

    #[derive(Any)]
    #[rune(item = ::toml::de)]
    pub(crate) struct Error {
        pub(crate) error: toml::de::Error,
    }

    impl Error {
        #[rune::function(protocol = DISPLAY_FMT)]
        pub(crate) fn display(&self, f: &mut Formatter) -> alloc::Result<()> {
            write!(f, "{}", self.error)
        }

        #[rune::function(protocol = DEBUG_FMT)]
        pub(crate) fn debug(&self, f: &mut Formatter) -> alloc::Result<()> {
            write!(f, "{:?}", self.error)
        }
    }

    impl From<toml::de::Error> for Error {
        fn from(error: toml::de::Error) -> Self {
            Self { error }
        }
    }
}

pub mod ser {
    //! Serializer types for the toml module.

    use rune::alloc;
    use rune::alloc::fmt::TryWrite;
    use rune::runtime::Formatter;
    use rune::{Any, ContextError, Module};

    pub fn module(_stdio: bool) -> Result<Module, ContextError> {
        let mut module = Module::with_crate_item("toml", ["ser"])?;
        module.ty::<Error>()?;
        module.function_meta(Error::display)?;
        module.function_meta(Error::debug)?;
        Ok(module)
    }

    #[derive(Any)]
    #[rune(item = ::toml::ser)]
    pub(crate) struct Error {
        pub(crate) error: toml::ser::Error,
    }

    impl Error {
        #[rune::function(protocol = DISPLAY_FMT)]
        pub(crate) fn display(&self, f: &mut Formatter) -> alloc::Result<()> {
            write!(f, "{}", self.error)
        }

        #[rune::function(protocol = DEBUG_FMT)]
        pub(crate) fn debug(&self, f: &mut Formatter) -> alloc::Result<()> {
            write!(f, "{:?}", self.error)
        }
    }

    impl From<toml::ser::Error> for Error {
        fn from(error: toml::ser::Error) -> Self {
            Self { error }
        }
    }
}

/// Convert bytes of TOML into a rune value.
#[rune::function]
fn from_bytes(bytes: &[u8]) -> Result<Result<Value, Value>, VmError> {
    let bytes = match std::str::from_utf8(bytes) {
        Ok(bytes) => bytes,
        Err(error) => return Ok(Err(rune::to_value(error)?)),
    };

    match toml::from_str(bytes).map_err(de::Error::from) {
        Ok(value) => Ok(Ok(value)),
        Err(error) => Ok(Err(rune::to_value(error)?)),
    }
}

/// Convert a string of TOML into a rune value.
#[rune::function]
fn from_string(string: &str) -> Result<Value, de::Error> {
    Ok(toml::from_str(string)?)
}

/// Convert any value to a toml string.
#[rune::function]
fn to_string(value: Value) -> alloc::Result<Result<String, ser::Error>> {
    Ok(Ok(String::try_from(nested_try!(toml::to_string(&value)))?))
}

/// Convert any value to toml bytes.
#[rune::function]
fn to_bytes(value: Value) -> alloc::Result<Result<Bytes, ser::Error>> {
    let string = String::try_from(nested_try!(toml::to_string(&value)))?;
    Ok(Ok(Bytes::from_vec(string.into_bytes())))
}
