pub(super) fn rune_memory_take(amount: usize) -> bool {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { crate::no_std::__rune_alloc_memory_take(amount) }
}

pub(super) fn rune_memory_release(amount: usize) {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { crate::no_std::__rune_alloc_memory_release(amount) }
}

pub(super) fn rune_memory_get() -> usize {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { crate::no_std::__rune_alloc_memory_get() }
}

pub(super) fn rune_memory_replace(value: usize) -> usize {
    // SAFETY: implementor is expected to have read the documentation and
    // implemented this correctly.
    unsafe { crate::no_std::__rune_alloc_memory_replace(value) }
}
