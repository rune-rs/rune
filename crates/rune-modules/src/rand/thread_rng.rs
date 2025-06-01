use rune::Any;

use super::OsError;

/// Access a fast, pre-initialized generator
///
/// This is a handle to the local [`ThreadRng`].
///
/// # Example
///
/// ```rune
/// // Using a local binding avoids an initialization-check on each usage:
/// let rng = rand::rng();
///
/// println!("True or false: {}", rng.random::<bool>());
/// println!("A simulated die roll: {}", rng.random_range::<u64>(1..=6));
/// ```
///
/// # Security
///
/// Refer to [`ThreadRng#Security`].
#[rune::function]
#[cfg(feature = "thread_rng")]
fn rng() -> ThreadRng {
    ThreadRng { inner: rand::rng() }
}

/// A reference to the thread-local generator
///
/// This type is a reference to a lazily-initialized thread-local generator. An
/// instance can be obtained via [`rand::rng()`][crate::rng()] or via
/// [`ThreadRng::default()`]. The handle cannot be passed between threads (is
/// not `Send` or `Sync`).
///
/// # Security
///
/// Security must be considered relative to a threat model and validation
/// requirements. The Rand project can provide no guarantee of fitness for
/// purpose. The design criteria for `ThreadRng` are as follows:
///
/// - Automatic seeding via [`OsRng`] and periodically thereafter (see
///   ([`ReseedingRng`] documentation). Limitation: there is no automatic
///   reseeding on process fork (see [below](#fork)).
/// - A rigorusly analyzed, unpredictable (cryptographic) pseudo-random
///   generator (see [the book on
///   security](https://rust-random.github.io/book/guide-rngs.html#security)).
///   The currently selected algorithm is ChaCha (12-rounds). See also
///   [`StdRng`] documentation.
/// - Not to leak internal state through [`Debug`] or serialization
///   implementations.
/// - No further protections exist to in-memory state. In particular, the
///   implementation is not required to zero memory on exit (of the process or
///   thread). (This may change in the future.)
/// - Be fast enough for general-purpose usage. Note in particular that
///   `ThreadRng` is designed to be a "fast, reasonably secure generator" (where
///   "reasonably secure" implies the above criteria).
///
/// We leave it to the user to determine whether this generator meets their
/// security requirements. For an alternative, see [`OsRng`].
///
/// # Fork
///
/// `ThreadRng` is not automatically reseeded on fork. It is recommended to
/// explicitly call [`ThreadRng::reseed`] immediately after a fork, for example:
///
/// ```ignore
/// fn do_fork() {
///     let pid = unsafe { libc::fork() };
///     if pid == 0 {
///         // Reseed ThreadRng in child processes:
///         rand::rng().reseed();
///     }
/// }
/// ```
///
/// Methods on `ThreadRng` are not reentrant-safe and thus should not be called
/// from an interrupt (e.g. a fork handler) unless it can be guaranteed that no
/// other method on the same `ThreadRng` is currently executing.
///
/// [`ReseedingRng`]: crate::rngs::ReseedingRng
/// [`StdRng`]: crate::rngs::StdRng
#[derive(Any)]
#[rune(item = ::rand)]
pub(super) struct ThreadRng {
    pub(super) inner: rand::rngs::ThreadRng,
}

impl ThreadRng {
    /// Immediately reseed the generator
    ///
    /// This discards any remaining random data in the cache.
    #[rune::function]
    pub(super) fn reseed(&mut self) -> Result<(), OsError> {
        Ok(self.inner.reseed()?)
    }
}
