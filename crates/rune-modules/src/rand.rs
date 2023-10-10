#![allow(dead_code)]
//! The native `rand` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.13.1", features = ["rand"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::rand::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! fn main() {
//!     let rng = rand::WyRand::new();
//!     let rand_int = rng.int();
//!     println(`Random int: {rand_int}`);
//!     let rand_int_range = rng.int_range(-100, 100);
//!     println(`Random int between -100 and 100: {rand_int_range}`);
//! }
//! ```

use nanorand::Rng;
use rune::{Any, ContextError, Module};
use rune::runtime::Value;

/// Construct the `rand` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("rand")?;

    module.ty::<WyRand>()?;
    module.function("new", WyRand::new).build_associated::<WyRand>()?;
    module.function("new_seed", WyRand::new_seed).build_associated::<WyRand>()?;
    module.associated_function("int", WyRand::int)?;
    module.associated_function("int_range", WyRand::int_range)?;

    module.ty::<Pcg64>()?;
    module.function("new", Pcg64::new).build_associated::<Pcg64>()?;
    module.function("new_seed", Pcg64::new_seed).build_associated::<Pcg64>()?;
    module.associated_function("int", Pcg64::int)?;
    module.associated_function("int_range", Pcg64::int_range)?;

    module.function("int", int).build()?;
    module.function("int_range", int_range).build()?;
    Ok(module)
}

#[derive(Any)]
#[rune(item = ::rand)]
struct WyRand {
    inner: nanorand::WyRand,
}

impl WyRand {
    /// Create a new RNG instance.
    fn new() -> Self {
        Self {
            inner: nanorand::WyRand::new(),
        }
    }

    /// Create a new RNG instance, using a custom seed.
    fn new_seed(seed: i64) -> Self {
        Self {
            inner: nanorand::WyRand::new_seed(seed as u64),
        }
    }

    /// Generate a random integer
    fn int(&mut self) -> Value {
        Value::Integer(self.inner.generate::<u64>() as i64)
    }

    /// Generate a random integer within the specified range
    fn int_range(&mut self, lower: i64, upper: i64) -> Value {
        Value::Integer(self.inner.generate_range(0..(upper - lower) as u64) as i64 + lower)
    }
}

#[derive(Any)]
#[rune(item = ::rand)]
struct Pcg64 {
    inner: nanorand::Pcg64,
}

impl Pcg64 {
    /// Create a new RNG instance.
    fn new() -> Self {
        Self {
            inner: nanorand::Pcg64::new(),
        }
    }

    /// Create a new RNG instance, using a custom seed.
    fn new_seed(seed: i64) -> Self {
        Self {
            inner: nanorand::Pcg64::new_seed(seed as u128),
        }
    }

    /// Generate a random integer
    fn int(&mut self) -> Value {
        Value::Integer(self.inner.generate::<u64>() as i64)
    }

    /// Generate a random integer within the specified range
    fn int_range(&mut self, lower: i64, upper: i64) -> Value {
        Value::Integer(self.inner.generate_range(0..(upper - lower) as u64) as i64 + lower)
    }
}

fn int() -> rune::support::Result<Value> {
    Ok(Value::Integer(
        nanorand::WyRand::new().generate::<u64>() as i64
    ))
}

fn int_range(lower: i64, upper: i64) -> rune::support::Result<Value> {
    Ok(Value::Integer(
        nanorand::WyRand::new().generate_range(0..(upper - lower) as u64) as i64 + lower,
    ))
}

#[cfg(test)]
mod tests {
    use super::{int, int_range};

    #[test]
    fn test_range_is_exclusive() {
        for _ in 0..100 {
            assert_eq!(rune::from_value::<i64>(int_range(0, 1).unwrap()).unwrap(), 0);
        }
    }

    #[test]
    fn test_range_can_be_negative() {
        for _ in 0..100 {
            assert_eq!(rune::from_value::<i64>(int_range(-2, -1).unwrap()).unwrap(), -2);
        }
    }

    #[test]
    fn test_int_is_properly_signed() {
        let mut any_negative = false;
        let mut any_positive = false;

        for _ in 0..100 {
            let v: i64 = rune::from_value(int().unwrap()).unwrap();
            any_negative = any_negative || v < 0;
            any_positive = any_positive || v > 0;
        }

        assert!(any_positive);
        assert!(any_negative);
    }
}
