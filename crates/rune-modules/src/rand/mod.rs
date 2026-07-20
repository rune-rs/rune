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
//! rune-modules = { version = "0.14.0", features = ["rand"] }
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
//!     let rng = rand::StdRng::try_from_sys_rng()?;
//!     let rand_int = rng.random::<u64>();
//!     println(`Random int: {rand_int}`);
//!     let rand_int_range = rng.random_range::<i64>(-100..100);
//!     println(`Random int between -100 and 100: {rand_int_range}`);
//! }
//! ```

#[macro_use]
mod macros;

#[cfg(feature = "sys_rng")]
mod sys_rng;
#[cfg(feature = "sys_rng")]
use self::sys_rng::SysRng;

#[cfg(any(feature = "thread_rng", feature = "sys_rng"))]
mod sys_error;
#[cfg(any(feature = "thread_rng", feature = "sys_rng"))]
use self::sys_error::SysError;

mod try_from_rng_error;
use self::try_from_rng_error::TryFromRngError;

mod small_rng;
use self::small_rng::SmallRng;

#[cfg(feature = "std_rng")]
mod std_rng;
#[cfg(feature = "std_rng")]
use self::std_rng::StdRng;

#[cfg(feature = "thread_rng")]
mod thread_rng;
#[cfg(feature = "thread_rng")]
use self::thread_rng::{rng, ThreadRng};

use rune::{ContextError, Module};

/// Construct the `rand` module.
#[rune::module(::rand)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    #[allow(unused_mut)]
    let mut m = Module::from_meta(module__meta)?;

    #[cfg(any(feature = "thread_rng", feature = "sys_rng"))]
    {
        m.ty::<SysError>()?;
        m.function_meta(SysError::display_fmt)?;
    }

    macro_rules! call_random {
        ($ty:ty, $example:expr) => {
            random! {
                m, $ty, $example,
                (random_u64, u64),
                (random_i64, i64),
                (random_char, char),
                (random_bool, bool),
            };

            random_ranges! {
                m, $ty, $example,
                (random_range_u64, u64, Value::as_integer::<u64>, 0..100),
                (random_range_i64, i64, Value::as_integer::<i64>, -100..100),
                (random_range_char, char, Value::as_char, 'a'..'z'),
            };
        };
    }

    #[cfg(feature = "sys_rng")]
    {
        m.ty::<SysRng>()?;
    }

    {
        m.ty::<SmallRng>()?;
        call_random!(SmallRng, "SmallRng::try_from_sys_rng()?");
        seedable_rng!(m, SmallRng);
    }

    #[cfg(feature = "std_rng")]
    {
        m.ty::<StdRng>()?;
        call_random!(StdRng, "StdRng::try_from_sys_rng()?");
        seedable_rng!(m, StdRng);
    }

    #[cfg(feature = "thread_rng")]
    {
        m.ty::<ThreadRng>()?;
        m.function_meta(ThreadRng::reseed)?;
        m.function_meta(rng)?;
        call_random!(ThreadRng, "rand::rng()");
    }

    Ok(m)
}
