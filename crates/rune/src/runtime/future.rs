use crate::compile::{InstallWith, Named};
use crate::runtime::{
    FromValue, Mut, RawMut, RawRef, RawStr, Ref, Shared, ToValue, UnsafeFromValue, Value, VmError,
    VmErrorKind, VmResult,
};
use pin_project::pin_project;
use std::fmt;
/// A future which can be unsafely polled.
use std::future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// dyn future alias.
type DynFuture = dyn future::Future<Output = VmResult<Value>> + 'static;

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
pub struct Future {
    future: Option<Pin<Box<DynFuture>>>,
}

impl Future {
    /// Construct a new wrapped future.
    pub fn new<T, O>(future: T) -> Self
    where
        T: 'static + future::Future<Output = VmResult<O>>,
        O: ToValue,
    {
        Self {
            future: Some(Box::pin(async move {
                let value = vm_try!(future.await);
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
    type Output = VmResult<Value>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<VmResult<Value>> {
        let this = self.get_mut();

        let future = match &mut this.future {
            Some(future) => future,
            None => {
                return Poll::Ready(VmResult::Err(VmError::from(VmErrorKind::FutureCompleted)));
            }
        };

        match future.as_mut().poll(cx) {
            Poll::Ready(result) => {
                this.future = None;
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl fmt::Debug for Future {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Future")
            .field("is_completed", &self.future.is_none())
            .finish_non_exhaustive()
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
    F: future::Future<Output = VmResult<Value>>,
{
    type Output = VmResult<(T, Value)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = this.future.poll(cx);

        match result {
            Poll::Ready(result) => match result {
                VmResult::Ok(value) => Poll::Ready(VmResult::Ok((*this.data, value))),
                VmResult::Err(error) => Poll::Ready(VmResult::Err(error)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

impl FromValue for Shared<Future> {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_shared_future()
    }
}

impl FromValue for Future {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_future()
    }
}

impl UnsafeFromValue for &Future {
    type Output = *const Future;
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let future = vm_try!(value.into_shared_future());
        let (future, guard) = Ref::into_raw(vm_try!(future.into_ref()));
        VmResult::Ok((future, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Future {
    type Output = *mut Future;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let future = vm_try!(value.into_shared_future());
        let future = vm_try!(future.into_mut());
        VmResult::Ok(Mut::into_raw(future))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl Named for Future {
    const BASE_NAME: RawStr = RawStr::from_str("Future");
}

impl InstallWith for Future {}
