use rune::Any;

/// A small-state, fast, non-crypto, non-portable PRNG
///
/// This is the "standard small" RNG, a generator with the following properties:
///
/// - Non-[portable]: any future library version may replace the algorithm and
///   results may be platform-dependent. (For a small portable generator, use
///   the [rand_pcg] or [rand_xoshiro] crate.)
/// - Non-cryptographic: output is easy to predict (insecure)
/// - [Quality]: statistically good quality
/// - Fast: the RNG is fast for both bulk generation and single values, with
///   consistent cost of method calls
/// - Fast initialization
/// - Small state: little memory usage (current state size is 16-32 bytes
///   depending on platform)
///
/// The current algorithm is `Xoshiro256PlusPlus` on 64-bit platforms and
/// `Xoshiro128PlusPlus` on 32-bit platforms. Both are also implemented by the
/// [rand_xoshiro] crate.
///
/// ## Seeding (construction)
///
/// This generator implements the [`SeedableRng`] trait. All methods are
/// suitable for seeding, but note that, even with a fixed seed, output is not
/// [portable]. Some suggestions:
///
/// To automatically seed with a unique seed, use [`SmallRng::from_rng`]:
///
/// ```rune
/// use rand::SmallRng;
///
/// let rng = rand::rng();
/// let rng = SmallRng::from_rng(rng);
/// ```
/// or [`SmallRng::from_os_rng`]:
/// ```rune
/// use rand::SmallRng;
///
/// let rng = SmallRng::from_os_rng();
/// ```
///
/// To use a deterministic integral seed, use `seed_from_u64`. This uses a
/// hash function internally to yield a (typically) good seed from any
/// input.
///
/// ```rune
/// use rand::SmallRng;
///
/// let rng = SmallRng::seed_from_u64(1);
/// ```
///
/// To seed deterministically from text or other input, use [`rand_seeder`].
///
/// See also [Seeding RNGs] in the book.
///
/// [portable]: https://rust-random.github.io/book/crate-reprod.html
/// [Seeding RNGs]: https://rust-random.github.io/book/guide-seeding.html
/// [Quality]: https://rust-random.github.io/book/guide-rngs.html#quality
/// [`StdRng`]: crate::rngs::StdRng
/// [rand_pcg]: https://crates.io/crates/rand_pcg
/// [rand_xoshiro]: https://crates.io/crates/rand_xoshiro
/// [`rand_chacha::ChaCha8Rng`]: https://docs.rs/rand_chacha/latest/rand_chacha/struct.ChaCha8Rng.html
/// [`rand_seeder`]: https://docs.rs/rand_seeder/latest/rand_seeder/
#[derive(Any)]
#[rune(item = ::rand)]
#[cfg(feature = "small_rng")]
pub(super) struct SmallRng {
    pub(super) inner: rand::rngs::SmallRng,
}
