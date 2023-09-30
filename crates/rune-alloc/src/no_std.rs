// In no-std environments, the implementor must define these functions.
//
// Normally these make use of thread-local storage, but if you want them to be
// completed disabled simply return dummy values or store it in static storage
// (if single threaded).
extern "C" {
    /// Take the given amount of memory from the current budget. Return `false`
    /// if the budget has been breached, or `true` otherwise.
    ///
    /// If this is called before `rune_memory_set` then it should usually just
    /// return `true`.
    pub(crate) fn __rune_alloc_memory_take(amount: usize) -> bool;

    /// Release the given amount of memory to the current budget.
    pub(crate) fn __rune_alloc_memory_release(amount: usize);

    /// Get the remaining memory budget for the current thread.
    pub(crate) fn __rune_alloc_memory_get() -> usize;

    /// Replace the memory budget for the current thread and return the one
    /// which was previously set.
    pub(crate) fn __rune_alloc_memory_replace(value: usize) -> usize;

    /// Abort the current process.
    ///
    /// In microcontrollers this might be implemented as an infinite loop.
    pub(crate) fn __rune_alloc_abort() -> !;
}

/// Terminates the process in an abnormal fashion.
///
/// The function will never return and will immediately terminate the current
/// process in a platform specific "abnormal" manner.
///
/// Note that because this function never returns, and that it terminates the
/// process, no destructors on the current stack or any other thread's stack
/// will be run.
pub fn abort() -> ! {
    // SAFETY: hook is always safe to call.
    unsafe { __rune_alloc_abort() }
}
