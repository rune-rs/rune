use rune::Any;

/// A strong, fast (amortized), non-portable RNG
///
/// This is the "standard" RNG, a generator with the following properties:
///
/// - Non-[portable]: any future library version may replace the algorithm and
///   results may be platform-dependent. (For a portable version, use the
///   [rand_chacha] crate directly.)
/// - [CSPRNG]: statistically good quality of randomness and [unpredictable]
/// - Fast ([amortized](https://en.wikipedia.org/wiki/Amortized_analysis)): the
///   RNG is fast for bulk generation, but the cost of method calls is not
///   consistent due to usage of an output buffer.
///
/// The current algorithm used is the ChaCha block cipher with 12 rounds. Please
/// see this relevant [rand issue] for the discussion. This may change as new
/// evidence of cipher security and performance becomes available.
///
/// ## Seeding (construction)
///
/// This generator implements the [`SeedableRng`] trait. Any method may be used,
/// but note that `seed_from_u64` is not suitable for usage where security is
/// important. Also note that, even with a fixed seed, output is not [portable].
///
/// Using a fresh seed **direct from the OS** is the most secure option:
///
/// ```rune
/// use rand::StdRng;
///
/// let rng = StdRng::try_from_os_rng()?;
/// ```
///
/// Seeding via [`rand::rng()`](crate::rng()) may be faster:
///
/// ```rune
/// use rand::StdRng;
///
/// let rng = rand::rng();
/// let rng = StdRng::from_rng(rng);
/// ```
///
/// Any [`SeedableRng`] method may be used, but note that `seed_from_u64` is not
/// suitable where security is required. See also [Seeding RNGs] in the book.
///
/// [portable]: https://rust-random.github.io/book/crate-reprod.html
/// [Seeding RNGs]: https://rust-random.github.io/book/guide-seeding.html
/// [unpredictable]: https://rust-random.github.io/book/guide-rngs.html#security
/// [CSPRNG]: https://rust-random.github.io/book/guide-gen.html#cryptographically-secure-pseudo-random-number-generator
/// [rand_chacha]: https://crates.io/crates/rand_chacha
/// [rand issue]: https://github.com/rust-random/rand/issues/932
#[derive(Any)]
#[rune(item = ::rand)]
pub(super) struct StdRng {
    pub(super) inner: rand::rngs::StdRng,
}
