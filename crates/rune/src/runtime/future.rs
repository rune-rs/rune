use core::fmt;
use core::future;
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll};

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::runtime::{ToValue, Value, VmError, VmErrorKind};
use crate::Any;

use pin_project::pin_project;

/// A virtual table for a type-erased future.
struct Vtable {
    poll: unsafe fn(*mut (), cx: &mut Context<'_>) -> Poll<Result<Value, VmError>>,
    drop: unsafe fn(*mut ()),
}

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
#[derive(Any)]
#[rune(crate)]
#[rune(item = ::std::future)]
pub struct Future {
    future: Option<NonNull<()>>,
    vtable: &'static Vtable,
}

impl Future {
    /// Construct a new wrapped future.
    pub(crate) fn new<T, O>(future: T) -> alloc::Result<Self>
    where
        T: 'static + future::Future<Output = Result<O, VmError>>,
        O: ToValue,
    {
        let (future, Global) = Box::into_raw_with_allocator(Box::try_new(future)?);

        let future = unsafe { NonNull::new_unchecked(future).cast() };

        Ok(Self {
            future: Some(future),
            vtable: &Vtable {
                poll: |future, cx| unsafe {
                    match Pin::new_unchecked(&mut *future.cast::<T>()).poll(cx) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(result) => match result {
                            Ok(result) => match result.to_value() {
                                Ok(value) => Poll::Ready(Ok(value)),
                                Err(err) => Poll::Ready(Err(err.into())),
                            },
                            Err(err) => Poll::Ready(Err(err)),
                        },
                    }
                },
                drop: |future| unsafe {
                    _ = Box::from_raw_in(future.cast::<T>(), Global);
                },
            },
        })
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
        unsafe {
            let this = self.get_unchecked_mut();

            let Some(future) = this.future else {
                return Poll::Ready(Err(VmError::new(VmErrorKind::FutureCompleted)));
            };

            match (this.vtable.poll)(future.as_ptr(), cx) {
                Poll::Ready(result) => {
                    this.future = None;
                    (this.vtable.drop)(future.as_ptr());
                    Poll::Ready(result)
                }
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

impl Drop for Future {
    fn drop(&mut self) {
        unsafe {
            if let Some(future) = self.future.take() {
                (self.vtable.drop)(future.as_ptr());
            }
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
    F: future::Future<Output = Result<Value, VmError>>,
{
    type Output = Result<(T, Value), VmError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = this.future.poll(cx);

        match result {
            Poll::Ready(result) => match result {
                Ok(value) => Poll::Ready(Ok((*this.data, value))),
                Err(error) => Poll::Ready(Err(error)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
