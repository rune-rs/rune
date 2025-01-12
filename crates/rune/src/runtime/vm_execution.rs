use core::fmt;
use core::future::Future;
use core::mem::{replace, take};
use core::pin::{pin, Pin};
use core::task::{ready, Context, Poll};

use ::rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::runtime::{
    Generator, GeneratorState, Output, RuntimeContext, Stream, Unit, Value, Vm, VmErrorKind,
    VmHalt, VmHaltInfo, VmResult,
};
use crate::shared::AssertSend;

use super::{future_vm_try, Awaited, VmDiagnostics};

use core::ptr;
use core::task::{RawWaker, RawWakerVTable, Waker};

const NOOP_RAW_WAKER: RawWaker = {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| NOOP_RAW_WAKER, |_| {}, |_| {}, |_| {});
    RawWaker::new(ptr::null(), &VTABLE)
};

static NOOP_WAKER: Waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };

/// The state of an execution. We keep track of this because it's important to
/// correctly interact with functions that yield (like generators and streams)
/// by initially just calling the function, then by providing a value pushed
/// onto the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
enum InnerExecutionState {
    /// The execution is running.
    Running,
    /// Execution has stopped running for yielding and expect output to be
    /// written to the given output.
    Yielded(Output),
}

impl fmt::Display for InnerExecutionState {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InnerExecutionState::Running => write!(f, "running"),
            InnerExecutionState::Yielded(out) => write!(f, "yielded({out})"),
        }
    }
}

#[derive(TryClone)]
#[try_clone(crate)]
pub(crate) struct VmExecutionState {
    pub(crate) context: Option<Arc<RuntimeContext>>,
    pub(crate) unit: Option<Arc<Unit>>,
}

/// The execution environment for a virtual machine.
///
/// When an execution is dropped, the stack of the stack of the head machine
/// will be cleared.
pub struct VmExecution<T = Vm>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// The current head vm which holds the execution.
    head: T,
    /// The state of an execution.
    state: InnerExecutionState,
    /// Indicates the current stack of suspended contexts.
    states: Vec<VmExecutionState>,
}

impl<T> VmExecution<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// Construct an execution from a virtual machine.
    #[inline]
    pub(crate) fn new(head: T) -> Self {
        Self {
            head,
            state: InnerExecutionState::Running,
            states: Vec::new(),
        }
    }

    /// Coerce the current execution into a generator if appropriate.
    ///
    /// ```
    /// use rune::Vm;
    /// use std::sync::Arc;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         pub fn main() {
    ///             yield 1;
    ///             yield 2;
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    ///
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let mut generator = vm.execute(["main"], ())?.into_generator();
    ///
    /// let mut n = 1i64;
    ///
    /// while let Some(value) = generator.next().into_result()? {
    ///     let value: i64 = rune::from_value(value)?;
    ///     assert_eq!(value, n);
    ///     n += 1;
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn into_generator(self) -> Generator<T> {
        Generator::from_execution(self)
    }

    /// Coerce the current execution into a stream if appropriate.
    ///
    /// ```
    /// use rune::Vm;
    /// use std::sync::Arc;
    ///
    /// # futures_executor::block_on(async move {
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         pub async fn main() {
    ///             yield 1;
    ///             yield 2;
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    ///
    /// let mut vm = Vm::without_runtime(Arc::new(unit));
    /// let mut stream = vm.execute(["main"], ())?.into_stream();
    ///
    /// let mut n = 1i64;
    ///
    /// while let Some(value) = stream.next().await.into_result()? {
    ///     let value: i64 = rune::from_value(value)?;
    ///     assert_eq!(value, n);
    ///     n += 1;
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// # })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn into_stream(self) -> Stream<T> {
        Stream::from_execution(self)
    }

    /// Get a reference to the current virtual machine.
    #[inline]
    pub fn vm(&self) -> &Vm {
        self.head.as_ref()
    }

    /// Get a mutable reference the current virtual machine.
    #[inline]
    pub fn vm_mut(&mut self) -> &mut Vm {
        self.head.as_mut()
    }

    /// Complete the current execution without support for async instructions.
    ///
    /// This will error if the execution is suspended through yielding.
    #[inline]
    pub fn async_complete(&mut self) -> Complete<'_, '_, T> {
        self.inner_poll(None, ResumeState::Default).into_complete()
    }

    /// Complete the current execution without support for async instructions.
    ///
    /// If any async instructions are encountered, this will error. This will
    /// also error if the execution is suspended through yielding.
    #[inline]
    pub fn complete(&mut self) -> VmResult<Value> {
        self.complete_with_diagnostics(None)
    }

    /// Complete the current execution without support for async instructions.
    ///
    /// If any async instructions are encountered, this will error. This will
    /// also error if the execution is suspended through yielding.
    #[inline]
    pub fn complete_with_diagnostics(
        &mut self,
        diagnostics: Option<&mut dyn VmDiagnostics>,
    ) -> VmResult<Value> {
        vm_try!(self.inner_resume(diagnostics, ResumeState::Resume(Value::empty()))).complete()
    }

    /// Resume the current execution with the given value and resume
    /// asynchronous execution.
    pub fn async_resume_with(&mut self, value: Value) -> Resume<'_, '_, T> {
        self.inner_poll(None, ResumeState::Resume(value))
            .into_resume()
    }

    /// Resume the current execution with support for async instructions.
    ///
    /// If the function being executed is a generator or stream this will resume
    /// it while returning a unit from the current `yield`.
    pub fn async_resume(&mut self) -> Resume<'_, '_, T> {
        self.async_resume_with_diagnostics(None)
    }

    /// Resume the current execution with support for async instructions.
    ///
    /// If the function being executed is a generator or stream this will resume
    /// it while returning a unit from the current `yield`.
    pub fn async_resume_with_diagnostics<'this, 'diag>(
        &'this mut self,
        diagnostics: Option<&'diag mut dyn VmDiagnostics>,
    ) -> Resume<'this, 'diag, T> {
        self.inner_poll(diagnostics, ResumeState::Resume(Value::empty()))
            .into_resume()
    }

    /// Resume the current execution with the given value and resume synchronous
    /// execution.
    #[tracing::instrument(skip_all, fields(?value))]
    pub fn resume_with(&mut self, value: Value) -> VmResult<GeneratorState> {
        vm_try!(self.inner_resume(None, ResumeState::Resume(value))).generator_state()
    }

    /// Resume the current execution without support for async instructions.
    ///
    /// If the function being executed is a generator or stream this will resume
    /// it while returning a unit from the current `yield`.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn resume(&mut self) -> VmResult<GeneratorState> {
        vm_try!(self.inner_resume(None, ResumeState::Resume(Value::empty()))).generator_state()
    }

    /// Resume the current execution without support for async instructions.
    ///
    /// If the function being executed is a generator or stream this will resume
    /// it while returning a unit from the current `yield`.
    ///
    /// If any async instructions are encountered, this will error.
    #[tracing::instrument(skip_all, fields(diagnostics=diagnostics.is_some()))]
    #[inline]
    pub fn resume_with_diagnostics(
        &mut self,
        diagnostics: Option<&mut dyn VmDiagnostics>,
    ) -> VmResult<GeneratorState> {
        vm_try!(self.inner_resume(diagnostics, ResumeState::Resume(Value::empty())))
            .generator_state()
    }

    #[inline]
    fn inner_resume(
        &mut self,
        mut diagnostics: Option<&mut dyn VmDiagnostics>,
        state: ResumeState,
    ) -> VmResult<ExecutionState> {
        pin!(self.inner_poll(diagnostics.take(), state)).once()
    }

    #[inline]
    fn inner_poll<'this, 'diag>(
        &'this mut self,
        diagnostics: Option<&'diag mut dyn VmDiagnostics>,
        state: ResumeState,
    ) -> Execution<'this, 'diag, T> {
        Execution {
            this: self,
            diagnostics,
            state,
        }
    }

    /// Push a virtual machine state onto the execution.
    #[tracing::instrument(skip_all)]
    #[inline]
    pub(crate) fn push_state(&mut self, state: VmExecutionState) -> VmResult<()> {
        tracing::trace!("pushing suspended state");
        let vm = self.head.as_mut();
        let context = state.context.map(|c| replace(vm.context_mut(), c));
        let unit = state.unit.map(|u| replace(vm.unit_mut(), u));
        vm_try!(self.states.try_push(VmExecutionState { context, unit }));
        VmResult::Ok(())
    }
}

impl VmExecution<&mut Vm> {
    /// Convert the current execution into one which owns its virtual machine.
    pub fn into_owned(self) -> VmExecution<Vm> {
        let stack = take(self.head.stack_mut());
        let head = Vm::with_stack(self.head.context().clone(), self.head.unit().clone(), stack);

        VmExecution {
            head,
            states: self.states,
            state: self.state,
        }
    }
}

/// A wrapper that makes [`VmExecution`] [`Send`].
///
/// This is accomplished by preventing any [`Value`] from escaping the [`Vm`].
/// As long as this is maintained, it is safe to send the execution across,
/// threads, and therefore schedule the future associated with the execution on
/// a thread pool like Tokio's through [tokio::spawn].
///
/// [tokio::spawn]: https://docs.rs/tokio/0/tokio/runtime/struct.Runtime.html#method.spawn
pub struct VmSendExecution(pub(crate) VmExecution<Vm>);

// Safety: we wrap all APIs around the [VmExecution], preventing values from
// escaping from contained virtual machine.
unsafe impl Send for VmSendExecution {}

impl VmSendExecution {
    /// Complete the current execution with support for async instructions.
    ///
    /// This requires that the result of the Vm is converted into a
    /// [crate::FromValue] that also implements [Send],  which prevents non-Send
    /// values from escaping from the virtual machine.
    pub fn async_complete(mut self) -> impl Future<Output = VmResult<Value>> + Send + 'static {
        let future = async move {
            let future = self.0.inner_poll(None, ResumeState::Default);
            vm_try!(future.await).complete()
        };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { AssertSend::new(future) }
    }

    /// Complete the current execution with support for async instructions.
    ///
    /// This requires that the result of the Vm is converted into a
    /// [crate::FromValue] that also implements [Send],  which prevents non-Send
    /// values from escaping from the virtual machine.
    pub async fn async_complete_with_diagnostics(
        mut self,
        diagnostics: Option<&mut dyn VmDiagnostics>,
    ) -> impl Future<Output = VmResult<Value>> + Send + '_ {
        let future = async move {
            let future = self.0.inner_poll(diagnostics, ResumeState::Default);
            vm_try!(future.await).complete()
        };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { AssertSend::new(future) }
    }
}

impl<T> TryClone for VmExecution<T>
where
    T: AsRef<Vm> + AsMut<Vm> + TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, rune_alloc::Error> {
        Ok(Self {
            head: self.head.try_clone()?,
            state: self.state,
            states: self.states.try_clone()?,
        })
    }
}

enum ResumeState {
    Default,
    Resume(Value),
    Await(Awaited),
}

/// The future when a virtual machine is resumed.
pub struct Resume<'a, 'b, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    inner: Execution<'a, 'b, T>,
}

impl<T> Future for Resume<'_, '_, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    type Output = VmResult<GeneratorState>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { Pin::map_unchecked_mut(self, |this| &mut this.inner) };
        let result = future_vm_try!(ready!(inner.poll(cx))).generator_state();
        Poll::Ready(result)
    }
}

/// The future when a value is produced.
pub struct Complete<'a, 'b, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    inner: Execution<'a, 'b, T>,
}

impl<T> Future for Complete<'_, '_, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    type Output = VmResult<Value>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { Pin::map_unchecked_mut(self, |this| &mut this.inner) };
        let result = future_vm_try!(ready!(inner.poll(cx))).complete();
        Poll::Ready(result)
    }
}

/// The full outcome of an execution.
///
/// This includes whether or not the execution was limited.
pub enum ExecutionState {
    Complete(Value),
    Yielded(Value),
    Limited,
}

impl ExecutionState {
    #[inline]
    fn complete(self) -> VmResult<Value> {
        match self {
            ExecutionState::Complete(value) => VmResult::Ok(value),
            ExecutionState::Yielded(..) => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            }),
            ExecutionState::Limited => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            }),
        }
    }

    #[inline]
    fn generator_state(self) -> VmResult<GeneratorState> {
        match self {
            ExecutionState::Complete(value) => VmResult::Ok(GeneratorState::Complete(value)),
            ExecutionState::Yielded(value) => VmResult::Ok(GeneratorState::Yielded(value)),
            ExecutionState::Limited => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            }),
        }
    }
}

/// The future when a virtual machine has been polled.
pub struct Execution<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    this: &'this mut VmExecution<T>,
    diagnostics: Option<&'diag mut dyn VmDiagnostics>,
    state: ResumeState,
}

impl<'this, 'diag, T> Execution<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// Consume and poll the future once.
    ///
    /// This will produce an error if the future is not ready, which implies
    /// that some async operation was involved that needed to be awaited.
    #[inline]
    pub fn once(self: Pin<&mut Self>) -> VmResult<ExecutionState> {
        let mut cx = Context::from_waker(&NOOP_WAKER);

        match self.poll(&mut cx) {
            Poll::Ready(result) => result,
            Poll::Pending => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Awaited,
            }),
        }
    }

    #[inline]
    fn into_resume(self) -> Resume<'this, 'diag, T> {
        Resume { inner: self }
    }

    #[inline]
    fn into_complete(self) -> Complete<'this, 'diag, T> {
        Complete { inner: self }
    }
}

impl<T> Future for Execution<'_, '_, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    type Output = VmResult<ExecutionState>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            this,
            diagnostics,
            state,
        } = self.get_mut();

        loop {
            let vm = this.head.as_mut();

            match state {
                ResumeState::Resume(..) => {
                    let ResumeState::Resume(value) = replace(state, ResumeState::Default) else {
                        unreachable!();
                    };

                    if let InnerExecutionState::Yielded(out) =
                        replace(&mut this.state, InnerExecutionState::Running)
                    {
                        future_vm_try!(out.store(vm.stack_mut(), value));
                    }
                }
                ResumeState::Await(awaited) => {
                    // SAFETY: We are polling from within a pinned type.
                    let awaited = unsafe { Pin::new_unchecked(awaited) };
                    future_vm_try!(ready!(awaited.poll(cx, vm)));
                    *state = ResumeState::Default;
                }
                ResumeState::Default => {
                    let result = vm
                        .run(match diagnostics {
                            Some(ref mut value) => Some(&mut **value),
                            None => None,
                        })
                        .with_vm(vm);

                    match future_vm_try!(result) {
                        VmHalt::Exited(addr) => {
                            let Some(state) = this.states.pop() else {
                                let value = match addr {
                                    Some(addr) => replace(
                                        future_vm_try!(vm.stack_mut().at_mut(addr)),
                                        Value::empty(),
                                    ),
                                    None => Value::unit(),
                                };

                                return Poll::Ready(VmResult::Ok(ExecutionState::Complete(value)));
                            };

                            if let Some(context) = state.context {
                                *vm.context_mut() = context;
                            }

                            if let Some(unit) = state.unit {
                                *vm.unit_mut() = unit;
                            }
                        }
                        VmHalt::Awaited(new) => {
                            *state = ResumeState::Await(new);
                        }
                        VmHalt::VmCall(vm_call) => {
                            future_vm_try!(vm_call.into_execution(this));
                        }
                        VmHalt::Yielded(addr, out) => {
                            let value = match addr {
                                Some(addr) => vm.stack().at(addr).clone(),
                                None => Value::unit(),
                            };

                            this.state = InnerExecutionState::Yielded(out);
                            return Poll::Ready(VmResult::Ok(ExecutionState::Yielded(value)));
                        }
                        VmHalt::Limited => {
                            return Poll::Ready(VmResult::Ok(ExecutionState::Limited));
                        }
                    }
                }
            }
        }
    }
}
