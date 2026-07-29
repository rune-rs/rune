use core::fmt;
use core::future;
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll};

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::runtime::vm_execution::VmResumeOwned;
use crate::runtime::{Dismantle, Handover, ToValue, Value, Vm, VmError, VmErrorKind, VmExecution};
use crate::Any;

use pin_project::pin_project;

/// A virtual table for a type-erased future.
struct Vtable {
    poll: unsafe fn(*mut (), cx: &mut Context<'_>) -> Poll<Result<Value, VmError>>,
    drop: unsafe fn(*mut ()),
    /// Hand over the values the future holds, for the futures whose values can
    /// be reached. `None` if they cannot be.
    dismantle: Option<unsafe fn(*mut (), &mut Handover<'_>)>,
    /// Test whether the future drives an execution which has yet to run.
    ///
    /// Set only for futures produced by an async call, since those are the ones
    /// whose execution the awaiting machine can take over.
    is_unstarted: Option<unsafe fn(*mut ()) -> bool>,
    /// Take the machine out of an unstarted execution. Set alongside
    /// `is_unstarted`, and only valid to call while it holds.
    take_vm: Option<unsafe fn(*mut ()) -> Vm>,
}

/// A type-erased future that can only be unsafely polled in combination with
/// the virtual machine that created it.
#[derive(Any)]
#[rune(crate)]
#[rune(item = ::std::future, dismantle)]
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
                dismantle: None,
                is_unstarted: None,
                take_vm: None,
            },
        })
    }

    /// Construct a future which drives the given execution.
    ///
    /// This is what an async call produces rather than a future built out of an
    /// `async` block, since the execution stays reachable through it. The
    /// values a suspended execution holds can then be handed over instead of
    /// being dropped in place, which is what keeps a chain of futures which
    /// were never awaited from being dropped one frame per level.
    pub(crate) fn from_execution(execution: VmExecution<Vm>) -> alloc::Result<Self> {
        let future = Box::try_new(VmResumeOwned::new(execution))?;
        let (future, Global) = Box::into_raw_with_allocator(future);

        let future = unsafe { NonNull::new_unchecked(future).cast() };

        Ok(Self {
            future: Some(future),
            vtable: &Vtable {
                poll: |future, cx| unsafe {
                    let future = Pin::new_unchecked(&mut *future.cast::<VmResumeOwned>());
                    future::Future::poll(future, cx)
                },
                drop: |future| unsafe {
                    _ = Box::from_raw_in(future.cast::<VmResumeOwned>(), Global);
                },
                dismantle: Some(|future, out| unsafe {
                    (*future.cast::<VmResumeOwned>()).dismantle(out)
                }),
                is_unstarted: Some(|future| unsafe {
                    (*future.cast::<VmResumeOwned>()).is_unstarted()
                }),
                take_vm: Some(|future| unsafe { (*future.cast::<VmResumeOwned>()).take_vm() }),
            },
        })
    }

    /// Take the machine out of this future, if it drives an execution which has
    /// yet to run a single instruction.
    ///
    /// The future is left completed, since what it was driving has been handed
    /// over. This is what lets a machine which awaits an async call splice that
    /// call in as an ordinary call frame rather than driving a nested machine -
    /// see [`Vm::splice_call`].
    ///
    /// [`Vm::splice_call`]: crate::runtime::Vm::splice_call
    pub(crate) fn take_unstarted_vm(&mut self) -> Option<Vm> {
        let (Some(future), Some(is_unstarted), Some(take_vm)) =
            (self.future, self.vtable.is_unstarted, self.vtable.take_vm)
        else {
            return None;
        };

        // SAFETY: The future has not completed, so it is still live, and we
        // hold it exclusively. Nothing borrows what it is made of in between
        // polls.
        unsafe {
            if !is_unstarted(future.as_ptr()) {
                return None;
            }

            let vm = take_vm(future.as_ptr());
            self.future = None;
            (self.vtable.drop)(future.as_ptr());
            Some(vm)
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

/// A future which was never awaited still holds everything the execution it
/// drives was working over, so futures nest just like a container does.
impl Dismantle for Future {
    fn dismantle(&mut self, out: &mut Handover<'_>) {
        let (Some(future), Some(dismantle)) = (self.future, self.vtable.dismantle) else {
            return;
        };

        // SAFETY: As above.
        unsafe { dismantle(future.as_ptr(), out) }
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
