use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// Internal helper struct to assert that a future is send.
#[pin_project::pin_project]
pub(crate) struct AssertSend<T>(#[pin] T);

impl<T> AssertSend<T> {
    /// Assert that the given inner value is `Send` by some external invariant.
    ///
    /// # Safety
    ///
    /// ProtocolCaller must assert that nothing is done with inner that violates it
    /// being `Send` at runtime.
    pub(crate) unsafe fn new(inner: T) -> Self {
        Self(inner)
    }
}

// Safety: we wrap all APIs around the [VmExecution], preventing values from
// escaping from contained virtual machine.
unsafe impl<T> Send for AssertSend<T> {}

impl<T> Future for AssertSend<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.0.poll(cx)
    }
}
