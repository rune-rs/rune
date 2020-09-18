//! Budgeting module for Runestick.
//!
//! This module contains methods which allows for limiting the execution of the
//! virtual machine to abide by the specified budget.
//!
//! By default the budget is disabled, but can be enabled by wrapping your
//! function call in [with].

use pin_project::pin_project;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

thread_local!(static BUDGET: Cell<usize> = Cell::new(usize::max_value()));

/// Wrap the given value with a budget.
///
/// The value can either be a function, after which you can use [Budget::call],
/// or it can be a [Future] which can be polled.
pub fn with<T>(budget: usize, value: T) -> Budget<T> {
    Budget { budget, value }
}

/// Take a ticket from the budget, indicating with `true` if the budget is
/// maintained
pub fn take() -> bool {
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

#[repr(transparent)]
struct BudgetGuard(usize);

impl Drop for BudgetGuard {
    fn drop(&mut self) {
        BUDGET.with(|tls| {
            tls.set(self.0);
        });
    }
}

/// A budgeted future.
#[pin_project]
pub struct Budget<T> {
    /// The current budget.
    budget: usize,
    /// The future being budgeted.
    #[pin]
    value: T,
}

impl<T, O> Budget<T>
where
    T: FnOnce() -> O,
{
    /// Call the wrapped function.
    pub fn call(self) -> O {
        BUDGET.with(|tls| {
            let _guard = BudgetGuard(tls.get());
            tls.set(self.budget);
            (self.value)()
        })
    }
}

impl<T> Future for Budget<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        BUDGET.with(|tls| {
            let _guard = BudgetGuard(tls.get());
            tls.set(*this.budget);
            let poll = this.value.poll(cx);
            *this.budget = tls.get();
            poll
        })
    }
}
