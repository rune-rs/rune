use crate::{
    FromValue, InstallWith, Mut, Named, RawMut, RawRef, RawStr, Ref, Shared, ToValue,
    UnsafeFromValue, Value, VmError,
};
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
    pub fn new<T, O>(future: T) -> Self
    where
        T: 'static + future::Future<Output = Result<O, VmError>>,
        O: ToValue,
    {
        Self {
            future: Some(Box::pin(async move {
                let value = future.await?;
                value.to_value()
            })),
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

impl FromValue for Shared<Future> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_shared_future()
    }
}

impl FromValue for Future {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_future()
    }
}

impl UnsafeFromValue for &Future {
    type Output = *const Future;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let future = value.into_shared_future()?;
        let (future, guard) = Ref::into_raw(future.into_ref()?);
        Ok((future, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Future {
    type Output = *mut Future;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let future = value.into_shared_future()?;
        Ok(Mut::into_raw(future.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl Named for Future {
    const BASE_NAME: RawStr = RawStr::from_str("Future");
}

impl InstallWith for Future {}
