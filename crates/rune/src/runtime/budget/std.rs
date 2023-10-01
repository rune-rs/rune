use core::cell::Cell;

std::thread_local!(static BUDGET: Cell<usize> = Cell::new(usize::MAX));

pub(super) fn rune_budget_take() -> bool {
    BUDGET.with(|tls| {
        let v = tls.get();
        tls.set(v.wrapping_sub(usize::from((v != usize::MAX) & (v != 0))));
        v != 0
    })
}

pub(super) fn rune_budget_get() -> usize {
    BUDGET.with(|tls| tls.get())
}

pub(super) fn rune_budget_replace(value: usize) -> usize {
    BUDGET.with(|tls| tls.replace(value))
}
