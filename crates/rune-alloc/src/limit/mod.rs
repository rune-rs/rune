//! Memory limits for Rune.
//!
//! This module contains methods which allows for limiting the memory use of the
//! virtual machine to abide by the specified memory limit.
//!
//! By default memory limits are disabled, but can be enabled by wrapping your
//! function call or future in [with].
//!
//! # Limitations
//!
//! Limiting is plugged in at the [Rust allocator level], and does not account
//! for allocator overhead. Allocator overhead comes about because an allocator
//! needs to use some extra system memory to perform internal bookkeeping.
//! Usually this should not be an issue, because the allocator overhead should
//! be a fragment of memory use. But the exact details would depend on the
//! [global allocator] used.
//!
//! As an example, see the [implementation notes for jemalloc].
//!
//! [implementation notes for jemalloc]:
//!     http://jemalloc.net/jemalloc.3.html#implementation_notes
//! [Rust allocator level]: https://doc.rust-lang.org/alloc/alloc/index.html
//! [global allocator]:
//!     https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html

#[cfg_attr(feature = "std", path = "std.rs")]
mod no_std;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

use crate::callable::Callable;

/// Something being budgeted.
///
/// See [`with`].
#[pin_project]
pub struct Memory<T> {
    /// The current limit.
    memory: usize,
    /// The thing being budgeted.
    #[pin]
    value: T,
}

/// Wrap the given value with a memory limit. Using a value of [`usize::MAX`]
/// effectively disables the memory limit.
///
/// The following things can be wrapped:
/// * A [`FnOnce`] closure, like `with(|| println!("Hello World")).call()`.
/// * A [`Future`], like `with(async { /* async work */ }).await`;
///
/// It's also possible to wrap other wrappers which implement [`Callable`].
///
/// See the [module level documentation] for more details.
///
/// [module level documentation]: crate::limit
///
/// # Examples
///
/// ```
/// use rune::alloc::limit;
/// use rune::alloc::Vec;
///
/// let f = limit::with(1024, || {
///     let mut vec = Vec::<u32>::try_with_capacity(256)?;
///
///     for n in 0..256u32 {
///         vec.try_push(n)?;
///     }
///
///     Ok::<_, rune::alloc::Error>(vec.into_iter().sum::<u32>())
/// });
///
/// let sum = f.call()?;
/// assert_eq!(sum, 32640);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// Breaching the limit. Note that this happens because while the vector is
/// growing it might both over-allocate, and hold onto two allocations
/// simultaneously.
///
/// ```
/// use rune::alloc::limit;
/// use rune::alloc::Vec;
///
/// let f = limit::with(1024, || {
///     let mut vec = Vec::<u32>::new();
///
///     for n in 0..256u32 {
///         vec.try_push(n)?;
///     }
///
///     Ok::<_, rune::alloc::Error>(vec.into_iter().sum::<u32>())
/// });
///
/// assert!(f.call().is_err());
/// ```
pub fn with<T>(memory: usize, value: T) -> Memory<T> {
    Memory { memory, value }
}

/// Get remaining memory that may be allocated.
///
/// # Examples
///
/// Example dealing with trait objects that were allocated externally:
///
/// ```
/// use rune::alloc::{Box, Vec};
/// use rune::alloc::limit;
/// use std::boxed::Box as StdBox;
///
/// assert_eq!(limit::get(), usize::MAX);
///
/// let b: StdBox<dyn Iterator<Item = u32>> = StdBox::new(1..3);
/// let mut b = Box::from_std(b)?;
/// assert_eq!(b.next(), Some(1));
/// assert_eq!(b.next(), Some(2));
/// assert_eq!(b.next(), None);
///
/// assert!(limit::get() < usize::MAX);
/// drop(b);
///
/// assert_eq!(limit::get(), usize::MAX);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub fn get() -> usize {
    self::no_std::rune_memory_get()
}

/// Take memory from the current budget.
#[inline(never)]
pub(crate) fn take(amount: usize) -> bool {
    self::no_std::rune_memory_take(amount)
}

/// Release memory from the current budget.
#[inline(never)]
pub(crate) fn release(amount: usize) {
    self::no_std::rune_memory_release(amount);
}

#[repr(transparent)]
struct MemoryGuard(usize);

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        let _ = self::no_std::rune_memory_replace(self.0);
    }
}

impl<T> Memory<T>
where
    T: Callable,
{
    /// Call the wrapped function, replacing the current budget and restoring it
    /// once the function call completes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::limit;
    /// use rune::alloc::{Box, Result};
    /// use rune::alloc::alloc::AllocError;
    ///
    /// const LIMIT: usize = 1024;
    ///
    /// fn doit() -> Result<Box<[u8; 256]>, AllocError> {
    ///     Box::try_new([0u8; 256])
    /// }
    ///
    /// fn limited() -> Result<()> {
    ///     assert_eq!(limit::get(), LIMIT);
    ///
    ///     // Hold onto a 256 byte allocation.
    ///     let b = doit()?;
    ///     assert_eq!(limit::get(), LIMIT - 256);
    ///
    ///     // Drop the allocation, making the memory available again.
    ///     drop(b);
    ///     assert_eq!(limit::get(), LIMIT);
    ///     Ok(())
    /// }
    ///
    /// let inner = limit::with(LIMIT, limited);
    ///
    /// assert_eq!(limit::get(), usize::MAX);
    /// inner.call()?;
    /// assert_eq!(limit::get(), usize::MAX);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Limit being restored after its been breached:
    ///
    /// ```
    /// use rune::alloc::limit;
    /// use rune::alloc::{Box, Result};
    /// use rune::alloc::alloc::AllocError;
    ///
    /// const LIMIT: usize = 128;
    ///
    /// fn doit() -> Result<Box<[u8; 256]>, AllocError> {
    ///     Box::try_new([0u8; 256])
    /// }
    ///
    /// fn limited() -> Result<()> {
    ///     assert_eq!(limit::get(), LIMIT);
    ///
    ///     // Fail to allocate since we don't have enough memory available.
    ///     assert!(doit().is_err());
    ///
    ///     assert_eq!(limit::get(), LIMIT);
    ///     Ok(())
    /// }
    ///
    /// let inner = limit::with(LIMIT, limited);
    ///
    /// assert_eq!(limit::get(), usize::MAX);
    /// inner.call()?;
    /// assert_eq!(limit::get(), usize::MAX);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn call(self) -> T::Output {
        Callable::call(self)
    }
}

impl<T> Callable for Memory<T>
where
    T: Callable,
{
    type Output = T::Output;

    #[inline]
    fn call(self) -> Self::Output {
        let _guard = MemoryGuard(self::no_std::rune_memory_replace(self.memory));
        self.value.call()
    }
}

/// Treat the current budget as a future, ensuring that the budget is suspended
/// and restored as necessary when the future is being polled.
///
/// # Examples
///
/// ```
/// use rune::alloc::limit;
/// use rune::alloc::{Box, Result};
/// use rune::alloc::alloc::AllocError;
///
/// const LIMIT: usize = 1024;
///
/// async fn doit() -> Result<Box<[u8; 256]>, AllocError> {
///     Box::try_new([0u8; 256])
/// }
///
/// async fn limited() -> Result<()> {
///     assert_eq!(limit::get(), LIMIT);
///
///     // Hold onto a 256 byte allocation.
///     let b = doit().await?;
///     assert_eq!(limit::get(), LIMIT - 256);
///
///     // Drop the allocation, making the memory available again.
///     drop(b);
///     assert_eq!(limit::get(), LIMIT);
///     Ok(())
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> rune::alloc::Result<()> {
/// let inner = limit::with(LIMIT, limited());
///
/// assert_eq!(limit::get(), usize::MAX);
/// inner.await?;
/// assert_eq!(limit::get(), usize::MAX);
/// # Ok::<_, rune::alloc::Error>(())
/// # }
/// ```
///
/// Limit being restored after its been breached:
///
/// ```
/// use rune::alloc::limit;
/// use rune::alloc::{Box, Result};
/// use rune::alloc::alloc::AllocError;
///
/// const LIMIT: usize = 128;
///
/// async fn doit() -> Result<Box<[u8; 256]>, AllocError> {
///     Box::try_new([0u8; 256])
/// }
///
/// async fn limited() -> Result<()> {
///     assert_eq!(limit::get(), LIMIT);
///
///     // Fail to allocate since we don't have enough memory available.
///     assert!(doit().await.is_err());
///
///     assert_eq!(limit::get(), LIMIT);
///     Ok(())
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> rune::alloc::Result<()> {
/// let inner = limit::with(LIMIT, limited());
///
/// assert_eq!(limit::get(), usize::MAX);
/// inner.await?;
/// assert_eq!(limit::get(), usize::MAX);
/// # Ok::<_, rune::alloc::Error>(())
/// # }
/// ```
impl<T> Future for Memory<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let _guard = MemoryGuard(self::no_std::rune_memory_replace(*this.memory));
        let poll = this.value.poll(cx);
        *this.memory = self::no_std::rune_memory_get();
        poll
    }
}
