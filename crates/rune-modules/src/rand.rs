//! The native `rand` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.16", features = ["rand"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::rand::module()?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! fn main() {
//!     let rng = rand::WyRand::new();
//!     let rand_int = rng.int();
//!     println(`Random int: {rand_int}`);
//!     let rand_int_range = rng.int_range(1, 100);
//!     println(`Random int between 1 and 100: {rand_int_range}`);
//!     let rand_byte = rng.byte();
//!     println(`Random byte: {rand_byte}`);
//!     let rand_byte_range = rng.byte_range(1, 64);
//!     println(`Random byte between 1 and 64: {rand_byte_range}`);
//! }
//! ```

use nanorand::{RandomGen, RandomRange, RNG};
use runestick::{Bytes, ContextError, Module, Value};

/// Construct the `rand` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["rand"]);
    module.ty::<WyRand>();
    module.ty::<Pcg64>();
    module.function(&["int"], int)?;
    module.function(&["int_range"], int_range)?;
    module.function(&["byte"], byte)?;
    module.function(&["byte_range"], byte_range)?;
    Ok(module)
}

#[derive(Debug, Any)]
struct WyRand {
    inner: nanorand::WyRand,
}

impl WyRand {
    /// Generate a random integer
    fn int(&mut self) -> Value {
        Value::Integer(self.inner.generate::<i64>())
    }

    /// Generate a random integer within the specified range
    fn int_range(&mut self, lower: i64, upper: i64) -> Value {
        Value::Integer(self.inner.generate_range::<i64>(lower, upper))
    }

    /// Generate a random byte
    fn byte(&mut self) -> Value {
        Value::Byte(self.inner.generate::<u8>())
    }

    /// Generate a random byte within the specified range
    fn byte_range(&mut self, lower: u8, upper: u8) -> Value {
        Value::Byte(self.inner.generate_range::<u8>(lower, upper))
    }
}

#[derive(Debug, Any)]
struct Pcg64 {
    inner: nanorand::Pcg64,
}

impl Pcg64 {
    /// Generate a random integer
    fn int(&mut self) -> Value {
        Value::Integer(self.inner.generate::<i64>())
    }

    /// Generate a random integer within the specified range
    fn int_range(&mut self, lower: i64, upper: i64) -> Value {
        Value::Integer(self.inner.generate_range::<i64>(lower, upper))
    }

    /// Generate a random byte
    fn byte(&mut self) -> Value {
        Value::Byte(self.inner.generate::<u8>())
    }

    /// Generate a random byte within the specified range
    fn byte_range(&mut self, lower: u8, upper: u8) -> Value {
        Value::Byte(self.inner.generate_range::<u8>(lower, upper))
    }
}

fn byte() -> runestick::Result<Value> {
    Ok(Value::Byte(WyRand::new().generate::<u8>()))
}

fn byte_range(lower: u8, upper: u8) -> runestick::Result<Value> {
    Ok(Value::Byte(
        WyRand::new().generate_range::<u8>(lower, upper),
    ))
}

fn int() -> runestick::Result<Value> {
    Ok(Value::Integer(WyRand::new().generate::<i64>()))
}

fn int_range(lower: i64, upper: i64) -> runestick::Result<Value> {
    Ok(Value::Integer(
        WyRand::new().generate_range::<i64>(lower, upper),
    ))
}
