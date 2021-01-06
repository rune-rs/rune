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
//! rune-modules = {version = "0.7.0", features = ["rand"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::rand::module(true)?)?;
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
//!     let rand_int_range = rng.int_range(-100, 100);
//!     println(`Random int between -100 and 100: {rand_int_range}`);
//! }
//! ```

use nanorand::RNG;
use runestick::{Any, ContextError, Module, Value};

/// Construct the `rand` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("rand");

    module.ty::<WyRand>()?;
    module.function(&["WyRand", "new"], WyRand::new)?;
    module.function(&["WyRand", "new_seed"], WyRand::new_seed)?;
    module.inst_fn("int", WyRand::int)?;
    module.inst_fn("int_range", WyRand::int_range)?;

    module.ty::<Pcg64>()?;
    module.function(&["Pcg64", "new"], Pcg64::new)?;
    module.function(&["Pcg64", "new_seed"], Pcg64::new_seed)?;
    module.inst_fn("int", Pcg64::int)?;
    module.inst_fn("int_range", Pcg64::int_range)?;

    module.function(&["int"], int)?;
    module.function(&["int_range"], int_range)?;

    Ok(module)
}

#[derive(Any)]
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
        Value::Integer(self.inner.generate_range::<u64>(0, (upper - lower) as u64) as i64 + lower)
    }
}

#[derive(Any)]
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
        Value::Integer(self.inner.generate_range::<u64>(0, (upper - lower) as u64) as i64 + lower)
    }
}

fn int() -> runestick::Result<Value> {
    Ok(Value::Integer(
        nanorand::WyRand::new().generate::<u64>() as i64
    ))
}

fn int_range(lower: i64, upper: i64) -> runestick::Result<Value> {
    Ok(Value::Integer(
        nanorand::WyRand::new().generate_range::<u64>(0, (upper - lower) as u64) as i64 + lower,
    ))
}
