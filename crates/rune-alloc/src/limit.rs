//! Memory limits for Rune.
//!
//! This module contains methods which allows for limiting the memory use of the
//! virtual machine to abide by the specified budget.
//!
//! By default memory limits are disabled, but can be enabled by wrapping your
//! function call or future in [with].

#[cfg_attr(feature = "std", path = "limit/std.rs")]
mod no_std;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

/// Something being budgeted.
#[pin_project]
pub struct Memory<T> {
    /// The current budget.
    budget: usize,
    /// The thing being budgeted.
    #[pin]
    value: T,
}

/// Wrap the given value with a memory limit.
///
/// # Examples
///
/// ```
/// use rune_alloc::limit;
/// use rune_alloc::Vec;
///
/// let f = limit::with(1024, || {
///     let mut vec = Vec::<u32>::try_with_capacity(256)?;
///
///     for n in 0..256u32 {
///         vec.try_push(n)?;
///     }
///
///     Ok::<_, rune_alloc::Error>(vec.into_iter().sum::<u32>())
/// });
///
/// let sum = f.call()?;
/// assert_eq!(sum, 32640);
/// # Ok::<_, rune_alloc::Error>(())
/// ```
///
/// Overloading the limit. Note that this happens because while the vector is
/// growing it might both over-allocate, and hold onto two allocations
/// simultaneously.
///
/// ```
/// use rune_alloc::limit;
/// use rune_alloc::Vec;
///
/// let f = limit::with(1024, || {
///     let mut vec = Vec::<u32>::new();
///
///     for n in 0..256u32 {
///         vec.try_push(n)?;
///     }
///
///     Ok::<_, rune_alloc::Error>(vec.into_iter().sum::<u32>())
/// });
///
/// assert!(f.call().is_err());
/// ```
pub fn with<T>(budget: usize, value: T) -> Memory<T> {
    Memory { budget, value }
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

impl<T, O> Memory<T>
where
    T: FnOnce() -> O,
{
    /// Call the wrapped function.
    pub fn call(self) -> O {
        let _guard = MemoryGuard(self::no_std::rune_memory_replace(self.budget));
        (self.value)()
    }
}

impl<T> Future for Memory<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let _guard = MemoryGuard(self::no_std::rune_memory_replace(*this.budget));
        let poll = this.value.poll(cx);
        *this.budget = self::no_std::rune_memory_get();
        poll
    }
}
