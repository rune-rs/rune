use rune::Any;

/// An interface over the operating-system's random data source
///
/// This is a zero-sized struct. It can be freely constructed with just `SysRng`.
///
/// The implementation is provided by the [`rand`] crate. Refer to
/// [`rand`] documentation for details.
///
/// This struct is available as `rand_core::SysRng` and as `rand::rngs::SysRng`.
/// In both cases, this requires the crate feature `sys_rng` or `std` (enabled by
/// default in `rand` but not in `rand_core`).
///
/// # Blocking and error handling
///
/// It is possible that when used during early boot the first call to `SysRng`
/// will block until the system's RNG is initialised. It is also possible
/// (though highly unlikely) for `SysRng` to fail on some platforms, most likely
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
/// use rand::{SmallRng, SysRng};
///
/// let rng = SmallRng::try_from_rng(SysRng)?;
/// let v = rng.random::<u64>();
/// ```
///
/// [`rand`]: https://docs.rs/rand
#[derive(Any)]
#[rune(item = ::rand, empty, constructor = SysRng::new)]
pub(super) struct SysRng {
    pub(super) inner: rand::rngs::SysRng,
}

impl SysRng {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            inner: rand::rngs::SysRng,
        }
    }
}
