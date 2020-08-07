use crate::reflection::ToValue;
use crate::value::ValuePtr;
use crate::vm::{Vm, VmError};
use std::fmt;
/// A future which can be unsafely polled.
use std::future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
pub struct Future {
    future: Option<NonNull<dyn future::Future<Output = Result<(), VmError>>>>,
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
    pub unsafe fn unsafe_new(
        future: *mut dyn future::Future<Output = Result<(), VmError>>,
    ) -> Self {
        Self {
            future: Some(NonNull::new_unchecked(future)),
        }
    }
}

impl ToValue for Future {
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        let slot = vm.slot_allocate(self);
        Ok(ValuePtr::Future(slot))
    }
}

impl fmt::Debug for Future {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Future")
    }
}

impl future::Future for Future {
    type Output = Result<(), VmError>;

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
