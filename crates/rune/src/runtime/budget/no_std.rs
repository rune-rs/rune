// In no-std environments, the implementor must define these functions.
//
// Normally these make use of thread-local storage, but if you want them to be
// completed disabled simply return dummy values or store it in static storage
// (if singlethreaded).
extern "C" {
    /// Take a ticket from the budget, returning a bool to indicate if we're
    /// still in budget. A value of `false` indicates that the budget has been
    /// breached.
    ///
    /// If this is called before `rune_budget_set` then it should usually just
    /// return `true`.
    pub(super) fn __rune_budget_take() -> bool;

    /// Get the current budget for the current thread.
    pub(super) fn __rune_budget_get() -> usize;

    /// Replace the current budget for the current thread and return the one
    /// which was previously set.
    pub(super) fn __rune_budget_replace(value: usize) -> usize;
}

pub(super) fn rune_budget_take() -> bool {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { __rune_budget_take() }
}

pub(super) fn rune_budget_get() -> usize {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { __rune_budget_get() }
}

pub(super) fn rune_budget_replace(value: usize) -> usize {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { __rune_budget_replace(value) }
}
