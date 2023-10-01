use core::fmt;
use core::future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate as rune;
use crate::alloc::{self, Box};
use crate::runtime::{ToValue, Value, VmErrorKind, VmResult};
use crate::Any;

use pin_project::pin_project;

/// dyn future alias.
type DynFuture = dyn future::Future<Output = VmResult<Value>> + 'static;

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
#[derive(Any)]
#[rune(builtin, static_type = FUTURE_TYPE, from_value = Value::into_future)]
pub struct Future {
    future: Option<Pin<Box<DynFuture>>>,
}

impl Future {
    /// Construct a new wrapped future.
    pub fn new<T, O>(future: T) -> alloc::Result<Self>
    where
        T: 'static + future::Future<Output = VmResult<O>>,
        O: ToValue,
    {
        // First construct a normal box, then coerce unsized.
        let b = Box::try_new(async move {
            let value = vm_try!(future.await);
            value.to_value()
        })?;

        // SAFETY: We know that the allocator the boxed used is `Global`, which
        // is compatible with the allocator used by the `std` box.
        unsafe {
            let (ptr, alloc) = Box::into_raw_with_allocator(b);
            // Our janky coerce unsized.
            let b: ::rust_alloc::boxed::Box<DynFuture> = ::rust_alloc::boxed::Box::from_raw(ptr);
            let b = ::rust_alloc::boxed::Box::into_raw(b);
            let b = Box::from_raw_in(b, alloc);

            // Second convert into one of our boxes, which ensures that memory is
            // being accounted for.
            Ok(Self {
                future: Some(Box::into_pin(b)),
            })
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
                return Poll::Ready(VmResult::err(VmErrorKind::FutureCompleted));
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
