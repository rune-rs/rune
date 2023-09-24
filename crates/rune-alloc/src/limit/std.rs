use core::cell::Cell;

std::thread_local!(static MEMORY: Cell<usize> = Cell::new(usize::MAX));

pub(super) fn rune_memory_take(amount: usize) -> bool {
    MEMORY.with(|tls| {
        let v = tls.get();

        if v == usize::MAX {
            true
        } else if v >= amount {
            tls.set(v - amount);
            true
        } else {
            tls.set(0);
            false
        }
    })
}

pub(super) fn rune_memory_release(amount: usize) {
    MEMORY.with(|tls| {
        let v = tls.get();

        if v != usize::MAX {
            tls.set(v.saturating_add(amount));
        }
    })
}

pub(super) fn rune_memory_get() -> usize {
    MEMORY.with(|tls| tls.get())
}

pub(super) fn rune_memory_replace(value: usize) -> usize {
    MEMORY.with(|tls| tls.replace(value))
}
