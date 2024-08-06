use core::cell::Cell;

std::thread_local!(static BUDGET: Cell<usize> = const { Cell::new(usize::MAX) });

pub(super) fn rune_budget_get() -> usize {
    BUDGET.with(|tls| tls.get())
}

pub(super) fn rune_budget_replace(value: usize) -> usize {
    BUDGET.with(|tls| tls.replace(value))
}
