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
//!     let rng = rand::StdRng::try_from_os_rng()?;
//!     let rand_int = rng.random::<u64>();
//!     println(`Random int: {rand_int}`);
//!     let rand_int_range = rng.random_range::<i64>(-100..100);
//!     println(`Random int between -100 and 100: {rand_int_range}`);
//! }
//! ```

#[cfg(any(feature = "small_rng", feature = "std_rng"))]
use rand::{Rng, SeedableRng};
#[cfg(feature = "os_rng")]
use rune::alloc::fmt::TryWrite;
#[cfg(feature = "os_rng")]
use rune::runtime::Formatter;
#[cfg(any(feature = "small_rng", feature = "std_rng", feature = "os_rng"))]
use rune::runtime::VmResult;
#[cfg(any(feature = "small_rng", feature = "std_rng"))]
use rune::runtime::{Range, RangeInclusive, TypeHash, Value};
#[cfg(any(feature = "small_rng", feature = "std_rng"))]
use rune::{vm_try, Any};
use rune::{ContextError, Module};

#[cfg(any(feature = "small_rng", feature = "std_rng"))]
macro_rules! random {
    ($m:ident, $ty:ty, $(($name:ident, $out:ty)),* $(,)?) => {
        $(
            #[doc = concat!(" Return a random `", stringify!($out), "` value via a standard uniform distribution.")]
            ///
            /// # Example
            ///
            /// ```rune
            #[doc = concat!(" use rand::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let rng = ", stringify!($ty), "::try_from_os_rng()?;")]
            #[doc = concat!(" let x = rng.random::<", stringify!($out), ">();")]
            /// println!("{x}");
            /// ```
            #[rune::function(instance, path = random<$out>)]
            fn $name(this: &mut $ty) -> $out {
                this.inner.random::<$out>()
            }

            $m.function_meta($name)?;
        )*
    }
}

#[cfg(any(feature = "small_rng", feature = "std_rng"))]
macro_rules! random_ranges {
    ($m:ident, $ty:ty, $(($name:ident, $out:ty, $as:path, $range:expr)),* $(,)?) => {
        $(
            #[doc = concat!(" Return a random `", stringify!($out), "` value via a standard uniform constrained with a range.")]
            ///
            /// # Example
            ///
            /// ```rune
            #[doc = concat!(" use rand::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let rng = ", stringify!($ty), "::try_from_os_rng()?;")]
            #[doc = concat!(" let x = rng.random_range::<", stringify!($out), ">(", stringify!($range), ");")]
            /// println!("{x}");
            /// ```
            #[rune::function(instance, path = random_range<$out>)]
            fn $name(this: &mut $ty, range: Value) -> VmResult<$out> {
                let value = match range.as_any() {
                    Some(value) => match value.type_hash() {
                        RangeInclusive::HASH => {
                            let range = vm_try!(value.borrow_ref::<RangeInclusive>());
                            let start = vm_try!($as(&range.start));
                            let end = vm_try!($as(&range.end));
                            this.inner.random_range(start..=end)
                        }
                        Range::HASH => {
                            let range = vm_try!(value.borrow_ref::<Range>());
                            let start = vm_try!($as(&range.start));
                            let end = vm_try!($as(&range.end));
                            this.inner.random_range(start..end)
                        }
                        _ => {
                            return VmResult::panic("unsupported range");
                        }
                    },
                    _ => {
                        return VmResult::panic("unsupported range");
                    }
                };

                VmResult::Ok(value)
            }

            $m.function_meta($name)?;
        )*
    }
}

/// Construct the `rand` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    #[allow(unused_mut)]
    let mut m = Module::with_crate("rand")?;

    #[cfg(feature = "os_rng")]
    {
        m.ty::<Error>()?;
        m.function_meta(Error::display_fmt)?;
    }

    #[cfg(any(feature = "small_rng", feature = "std_rng"))]
    macro_rules! call_random {
        ($ty:ty) => {
            random! {
                m, $ty,
                (random_u64, u64),
                (random_i64, i64),
                (random_char, char),
            };

            random_ranges! {
                m,
                $ty,
                (random_range_u64, u64, Value::as_integer::<u64>, 0..100),
                (random_range_i64, i64, Value::as_integer::<i64>, -100..100),
                (random_range_char, char, Value::as_char, 'a'..'z'),
            };
        };
    }

    #[cfg(feature = "small_rng")]
    {
        m.ty::<SmallRng>()?;
        #[cfg(feature = "os_rng")]
        m.function_meta(SmallRng::from_os_rng)?;
        #[cfg(feature = "os_rng")]
        m.function_meta(SmallRng::try_from_os_rng)?;
        m.function_meta(SmallRng::from_seed)?;
        m.function_meta(SmallRng::seed_from_u64)?;
        call_random!(SmallRng);
    }

    #[cfg(feature = "std_rng")]
    {
        m.ty::<StdRng>()?;
        #[cfg(feature = "os_rng")]
        m.function_meta(StdRng::from_os_rng)?;
        #[cfg(feature = "os_rng")]
        m.function_meta(StdRng::try_from_os_rng)?;
        m.function_meta(StdRng::from_seed)?;
        m.function_meta(StdRng::seed_from_u64)?;
        call_random!(StdRng);
    }

    Ok(m)
}

/// An error returned by methods in the `http` module.
#[derive(Debug, Any)]
#[rune(item = ::rand)]
#[cfg(feature = "os_rng")]
pub struct Error {
    inner: getrandom::Error,
}

#[cfg(feature = "os_rng")]
impl From<getrandom::Error> for Error {
    #[inline]
    fn from(inner: getrandom::Error) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "os_rng")]
impl Error {
    /// Write a display representation the error.
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self.inner)
    }
}

#[derive(Any)]
#[rune(item = ::rand)]
#[cfg(feature = "small_rng")]
struct SmallRng {
    inner: rand::rngs::SmallRng,
}

#[cfg(feature = "small_rng")]
impl SmallRng {
    /// Creates a new instance of the RNG seeded via [`getrandom`].
    ///
    /// This method is the recommended way to construct non-deterministic PRNGs
    /// since it is convenient and secure.
    ///
    /// Note that this method may panic on (extremely unlikely) [`getrandom`]
    /// errors. If it's not desirable, use the [`try_from_os_rng`] method
    /// instead.
    ///
    /// # Panics
    ///
    /// If [`getrandom`] is unable to provide secure entropy this method will
    /// panic.
    ///
    /// [`getrandom`]: https://docs.rs/getrandom
    /// [`try_from_os_rng`]: StdRng::try_from_os_rng
    #[rune::function(path = SmallRng::from_os_rng)]
    #[cfg(feature = "os_rng")]
    fn from_os_rng() -> VmResult<Self> {
        match rand::rngs::SmallRng::try_from_os_rng() {
            Ok(inner) => VmResult::Ok(Self { inner }),
            Err(e) => VmResult::panic(e),
        }
    }

    /// Creates a new instance of the RNG seeded via [`getrandom`] without
    /// unwrapping potential [`getrandom`] errors.
    ///
    /// [`getrandom`]: https://docs.rs/getrandom
    #[rune::function(path = SmallRng::try_from_os_rng)]
    #[cfg(feature = "os_rng")]
    fn try_from_os_rng() -> Result<Self, Error> {
        match rand::rngs::SmallRng::try_from_os_rng() {
            Ok(inner) => Ok(Self { inner }),
            Err(inner) => Err(Error { inner }),
        }
    }

    /// Create a new PRNG using the given seed.
    ///
    /// PRNG implementations are allowed to assume that bits in the seed are
    /// well distributed. That means usually that the number of one and zero
    /// bits are roughly equal, and values like 0, 1 and (size - 1) are
    /// unlikely. Note that many non-cryptographic PRNGs will show poor quality
    /// output if this is not adhered to. If you wish to seed from simple
    /// numbers, use [`seed_from_u64`] instead.
    ///
    /// All PRNG implementations should be reproducible unless otherwise noted:
    /// given a fixed `seed`, the same sequence of output should be produced on
    /// all runs, library versions and architectures (e.g. check endianness).
    /// Any "value-breaking" changes to the generator should require bumping at
    /// least the minor version and documentation of the change.
    ///
    /// It is not required that this function yield the same state as a
    /// reference implementation of the PRNG given equivalent seed; if necessary
    /// another constructor replicating behaviour from a reference
    /// implementation can be added.
    ///
    /// PRNG implementations should make sure `from_seed` never panics. In the
    /// case that some special values (like an all zero seed) are not viable
    /// seeds it is preferable to map these to alternative constant value(s),
    /// for example `0xBAD5EEDu32` or `0x0DDB1A5E5BAD5EEDu64` ("odd biases? bad
    /// seed"). This is assuming only a small number of values must be rejected.
    ///
    /// [`seed_from_u64`]: SmallRng::seed_from_u64
    #[rune::function(path = Self::from_seed)]
    fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            inner: rand::rngs::SmallRng::from_seed(seed),
        }
    }

    /// Create a new PRNG using a `u64` seed.
    ///
    /// This is a convenience-wrapper around `from_seed` to allow construction
    /// of any `SeedableRng` from a simple `u64` value. It is designed such that
    /// low Hamming Weight numbers like 0 and 1 can be used and should still
    /// result in good, independent seeds to the PRNG which is returned.
    ///
    /// This **is not suitable for cryptography**, as should be clear given that
    /// the input size is only 64 bits.
    ///
    /// Implementations for PRNGs *may* provide their own implementations of
    /// this function, but the default implementation should be good enough for
    /// all purposes. *Changing* the implementation of this function should be
    /// considered a value-breaking change.
    #[rune::function(path = Self::seed_from_u64)]
    fn seed_from_u64(state: u64) -> Self {
        Self {
            inner: rand::rngs::SmallRng::seed_from_u64(state),
        }
    }
}

#[derive(Any)]
#[rune(item = ::rand)]
#[cfg(feature = "std_rng")]
struct StdRng {
    inner: rand::rngs::StdRng,
}

#[cfg(feature = "std_rng")]
impl StdRng {
    /// Creates a new instance of the RNG seeded via [`getrandom`].
    ///
    /// This method is the recommended way to construct non-deterministic PRNGs
    /// since it is convenient and secure.
    ///
    /// Note that this method may panic on (extremely unlikely) [`getrandom`]
    /// errors. If it's not desirable, use the [`try_from_os_rng`] method
    /// instead.
    ///
    /// # Panics
    ///
    /// If [`getrandom`] is unable to provide secure entropy this method will
    /// panic.
    ///
    /// [`getrandom`]: https://docs.rs/getrandom
    /// [`try_from_os_rng`]: StdRng::try_from_os_rng
    #[rune::function(path = StdRng::from_os_rng)]
    #[cfg(feature = "os_rng")]
    fn from_os_rng() -> VmResult<Self> {
        match rand::rngs::StdRng::try_from_os_rng() {
            Ok(inner) => VmResult::Ok(Self { inner }),
            Err(e) => VmResult::panic(e),
        }
    }

    /// Creates a new instance of the RNG seeded via [`getrandom`] without
    /// unwrapping potential [`getrandom`] errors.
    ///
    /// [`getrandom`]: https://docs.rs/getrandom
    #[rune::function(path = StdRng::try_from_os_rng)]
    #[cfg(feature = "os_rng")]
    fn try_from_os_rng() -> Result<Self, Error> {
        match rand::rngs::StdRng::try_from_os_rng() {
            Ok(inner) => Ok(Self { inner }),
            Err(inner) => Err(Error { inner }),
        }
    }

    /// Create a new PRNG using the given seed.
    ///
    /// PRNG implementations are allowed to assume that bits in the seed are
    /// well distributed. That means usually that the number of one and zero
    /// bits are roughly equal, and values like 0, 1 and (size - 1) are
    /// unlikely. Note that many non-cryptographic PRNGs will show poor quality
    /// output if this is not adhered to. If you wish to seed from simple
    /// numbers, use [`seed_from_u64`] instead.
    ///
    /// All PRNG implementations should be reproducible unless otherwise noted:
    /// given a fixed `seed`, the same sequence of output should be produced on
    /// all runs, library versions and architectures (e.g. check endianness).
    /// Any "value-breaking" changes to the generator should require bumping at
    /// least the minor version and documentation of the change.
    ///
    /// It is not required that this function yield the same state as a
    /// reference implementation of the PRNG given equivalent seed; if necessary
    /// another constructor replicating behaviour from a reference
    /// implementation can be added.
    ///
    /// PRNG implementations should make sure `from_seed` never panics. In the
    /// case that some special values (like an all zero seed) are not viable
    /// seeds it is preferable to map these to alternative constant value(s),
    /// for example `0xBAD5EEDu32` or `0x0DDB1A5E5BAD5EEDu64` ("odd biases? bad
    /// seed"). This is assuming only a small number of values must be rejected.
    ///
    /// [`seed_from_u64`]: StdRng::seed_from_u64
    #[rune::function(path = Self::from_seed)]
    fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            inner: rand::rngs::StdRng::from_seed(seed),
        }
    }

    /// Create a new PRNG using a `u64` seed.
    ///
    /// This is a convenience-wrapper around `from_seed` to allow construction
    /// of any `SeedableRng` from a simple `u64` value. It is designed such that
    /// low Hamming Weight numbers like 0 and 1 can be used and should still
    /// result in good, independent seeds to the PRNG which is returned.
    ///
    /// This **is not suitable for cryptography**, as should be clear given that
    /// the input size is only 64 bits.
    ///
    /// Implementations for PRNGs *may* provide their own implementations of
    /// this function, but the default implementation should be good enough for
    /// all purposes. *Changing* the implementation of this function should be
    /// considered a value-breaking change.
    #[rune::function(path = Self::seed_from_u64)]
    fn seed_from_u64(state: u64) -> Self {
        Self {
            inner: rand::rngs::StdRng::seed_from_u64(state),
        }
    }
}
