use crate::{Shared, ToValue, Value, ValueError, VmError};
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
}

impl future::Future for Future {
    type Output = Result<Value, VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Value, VmError>> {
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

/// Future wrapper used to keep track of associated data.
#[pin_project]
pub struct SelectFuture<T, F> {
    data: T,
    #[pin]
    future: F,
}

impl<T, F> SelectFuture<T, F> {
    /// Construct a new select future.
    pub fn new(data: T, future: F) -> Self {
        Self { data, future }
    }
}

impl<T, F> future::Future for SelectFuture<T, F>
where
    T: Copy,
    F: future::Future<Output = Result<Value, VmError>>,
{
    type Output = Result<(T, Value), VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = this.future.poll(cx);

        match result {
            Poll::Ready(result) => match result {
                Ok(value) => Poll::Ready(Ok((*this.data, value))),
                Err(e) => Poll::Ready(Err(e)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
