use crate::{AssertInVm, OwnedMut, Shared, ToValue, Value, ValueError, VmError};
use pin_project::pin_project;
use std::fmt;
/// A future which can be unsafely polled.
use std::future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// dyn future alias.
type DynFuture = dyn future::Future<Output = Result<Value, VmError>> + 'static;

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
pub struct Future {
    future: Option<Pin<Box<DynFuture>>>,
}

impl Future {
    /// Construct a new wrapped future.
    pub fn new<T>(future: T) -> Self
    where
        T: 'static + future::Future<Output = Result<Value, VmError>>,
    {
        Self {
            future: Some(Box::pin(future)),
        }
    }

    /// Check if future is completed.
    ///
    /// This will prevent it from being used in a select expression.
    pub fn is_completed(&self) -> bool {
        self.future.is_none()
    }

    /// Poll the given future.
    ///
    /// # Safety
    ///
    /// Polling is unsafe, and requires a proof obligation that we are not
    /// polling outside of the virtual machine.
    ///
    /// This can be done by wrapping a [StrongMut] in [AssertInVm] using
    /// [StrongMut::assert_in_vm].
    pub unsafe fn unsafe_poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Value, VmError>> {
        let this = self.get_mut();
        let mut future = this.future.take().expect("futures can only be polled once");

        match future.as_mut().poll(cx) {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => {
                this.future = Some(future);
                Poll::Pending
            }
        }
    }
}

impl ToValue for Future {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Future(Shared::new(self)))
    }
}

impl fmt::Debug for Future {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Future")
            .field("is_completed", &self.future.is_none())
            .finish()
    }
}

/// Future wrapper, used when selecting over a branch of futures.
#[pin_project]
pub struct SelectFuture<F> {
    #[pin]
    future: F,
    index: usize,
}

impl<F> SelectFuture<F> {
    /// Construct a new select future.
    pub fn new(future: F, index: usize) -> Self {
        Self { future, index }
    }
}

impl<F> future::Future for SelectFuture<F>
where
    F: future::Future<Output = Result<Value, VmError>>,
{
    type Output = Result<(usize, Value), VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = this.future.poll(cx);

        match result {
            Poll::Ready(result) => match result {
                Ok(value) => Poll::Ready(Ok((*this.index, value))),
                Err(e) => Poll::Ready(Err(e)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

impl future::Future for AssertInVm<OwnedMut<Future>> {
    type Output = Result<Value, VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: we have a proof obligation that we are not being polled
        // outside of the virtual machine by being wrapped in an `AssertInVm`.
        unsafe {
            let future = self.map_unchecked_mut(|this| &mut *this.inner);
            future.unsafe_poll(cx)
        }
    }
}
