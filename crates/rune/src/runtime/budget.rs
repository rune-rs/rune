//! Budgeting module for Runestick.
//!
//! This module contains methods which allows for limiting the execution of the
//! virtual machine to abide by the specified budget.
//!
//! By default the budget is disabled, but can be enabled by wrapping your
//! function call in [with].

#[cfg_attr(feature = "std", path = "budget/std.rs")]
mod no_std;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

#[cfg(feature = "std")]
#[cfg(not(feature = "std"))]
static BUDGET: Cell<usize> = Cell::new(usize::max_value());

/// Something being budgeted.
#[pin_project]
pub struct Budget<T> {
    /// The current budget.
    budget: usize,
    /// The thing being budgeted.
    #[pin]
    value: T,
}

/// Wrap the given value with a budget.
pub fn with<T>(budget: usize, value: T) -> Budget<T> {
    tracing::trace!(?budget);
    Budget { budget, value }
}

/// Take a ticket from the budget, indicating with `true` if the budget is
/// maintained
#[inline(never)]
pub(crate) fn take() -> bool {
    self::no_std::rune_budget_take()
}

#[repr(transparent)]
struct BudgetGuard(usize);

impl Drop for BudgetGuard {
    fn drop(&mut self) {
        let _ = self::no_std::rune_budget_replace(self.0);
    }
}

impl<T, O> Budget<T>
where
    T: FnOnce() -> O,
{
    /// Call the wrapped function.
    pub(crate) fn call(self) -> O {
        let _guard = BudgetGuard(self::no_std::rune_budget_replace(self.budget));
        (self.value)()
    }
}

impl<T> Future for Budget<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let _guard = BudgetGuard(self::no_std::rune_budget_replace(*this.budget));
        let poll = this.value.poll(cx);
        *this.budget = self::no_std::rune_budget_get();
        poll
    }
}
