use core::cell::Cell;

std::thread_local!(static BUDGET: Cell<usize> = Cell::new(usize::max_value()));

pub(super) fn rune_budget_take() -> bool {
    BUDGET.with(|tls| {
        let v = tls.get();

        if v == usize::max_value() {
            true
        } else if v == 0 {
            false
        } else {
            tls.set(v - 1);
            true
        }
    })
}

pub(super) fn rune_budget_get() -> usize {
    BUDGET.with(|tls| tls.get())
}

pub(super) fn rune_budget_replace(value: usize) -> usize {
    BUDGET.with(|tls| tls.replace(value))
}
