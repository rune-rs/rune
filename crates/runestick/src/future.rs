use crate::reflection::ToValue;
use crate::shared::Shared;
use crate::value::{Value, ValueError};
use crate::vm::VmError;
use std::fmt;
/// A future which can be unsafely polled.
use std::future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
pub struct Future {
    future: Option<NonNull<dyn future::Future<Output = Result<Value, VmError>>>>,
}

impl Future {
    /// Construct a new future wrapper, ignoring the constraints of the incoming
    /// lifetime.
    ///
    /// This object will take ownership of the provided future, and it will be
    /// dropped when this future wrapper is dropped unless it's been consumed.
    ///
    /// # Safety
    ///
    /// A future constructed through this must **only** be polled while any
    /// data it references is **live**.
    pub unsafe fn new_unchecked(
        future: *mut dyn future::Future<Output = Result<Value, VmError>>,
    ) -> Self {
        Self {
            future: Some(NonNull::new_unchecked(future)),
        }
    }

    /// Check if future is completed.
    ///
    /// This will prevent it from being used in a select expression.
    pub fn is_completed(&self) -> bool {
        self.future.is_none()
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

impl future::Future for Future {
    type Output = Result<Value, VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let mut future = this.future.take().expect("futures can only be polled once");

        // Safety: We can only get here through unsafe constructors which have
        // certain safety requirements that need to be upheld.
        //
        // See [unsafe_new].
        unsafe {
            match Pin::new_unchecked(future.as_mut()).poll(cx) {
                Poll::Ready(result) => {
                    // NB: dropping the inner trait object.
                    let _ = Box::from_raw(future.as_ptr());
                    Poll::Ready(result)
                }
                Poll::Pending => {
                    this.future = Some(future);
                    Poll::Pending
                }
            }
        }
    }
}

impl Drop for Future {
    fn drop(&mut self) {
        if let Some(future) = self.future.take() {
            let _ = unsafe { Box::from_raw(future.as_ptr()) };
        }
    }
}

/// Future wrapper, used when selecting over a branch of futures.
pub struct SelectFuture {
    future: *mut Future,
    index: usize,
}

impl SelectFuture {
    /// Construct a new select future.
    ///
    /// # Safety
    ///
    /// This polls over a raw future, and the caller must ensure that any
    /// references held by the underlying future must be live while it is being
    /// polled.
    pub unsafe fn new_unchecked(future: *mut Future, index: usize) -> Self {
        Self { future, index }
    }
}

impl future::Future for SelectFuture {
    type Output = Result<(usize, Value), VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Safety: this future can only be constructed through an unsafe
        // constructor, and must therefore abide by its safety requirements.
        let result = unsafe { Pin::new_unchecked(&mut *this.future).poll(cx) };

        match result {
            Poll::Ready(result) => match result {
                Ok(value) => Poll::Ready(Ok((this.index, value))),
                Err(e) => Poll::Ready(Err(e)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
