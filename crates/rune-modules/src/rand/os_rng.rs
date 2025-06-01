use rune::Any;

/// An interface over the operating-system's random data source
///
/// This is a zero-sized struct. It can be freely constructed with just `OsRng`.
///
/// The implementation is provided by the [getrandom] crate. Refer to
/// [getrandom] documentation for details.
///
/// This struct is available as `rand_core::OsRng` and as `rand::rngs::OsRng`.
/// In both cases, this requires the crate feature `os_rng` or `std` (enabled by
/// default in `rand` but not in `rand_core`).
///
/// # Blocking and error handling
///
/// It is possible that when used during early boot the first call to `OsRng`
/// will block until the system's RNG is initialised. It is also possible
/// (though highly unlikely) for `OsRng` to fail on some platforms, most likely
/// due to system mis-configuration.
///
/// After the first successful call, it is highly unlikely that failures or
/// significant delays will occur (although performance should be expected to be
/// much slower than a user-space
/// [PRNG](https://rust-random.github.io/book/guide-gen.html#pseudo-random-number-generators)).
///
/// # Usage example
///
/// ```rune
/// use rand::{SmallRng, OsRng};
///
/// let rng = SmallRng::try_from_rng(OsRng)?;
/// let v = rng.random::<u64>();
/// ```
///
/// [getrandom]: https://crates.io/crates/getrandom
#[derive(Any)]
#[rune(item = ::rand, fields = empty)]
pub(super) struct OsRng {
    pub(super) inner: rand::rngs::OsRng,
}

impl OsRng {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            inner: rand::rngs::OsRng,
        }
    }
}
