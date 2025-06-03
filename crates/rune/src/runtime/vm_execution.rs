use core::fmt;
use core::future::Future;
use core::mem::{replace, take};
use core::pin::{pin, Pin};
use core::task::{ready, Context, Poll, RawWaker, RawWakerVTable, Waker};

use rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::runtime::budget::Budget;
use crate::runtime::{budget, Awaited};
use crate::shared::AssertSend;
use crate::{async_vm_try, vm_try};

use super::{
    Generator, GeneratorState, InstAddress, Output, RuntimeContext, Stream, Unit, Value, Vm,
    VmDiagnostics, VmErrorKind, VmHalt, VmHaltInfo, VmResult,
};

static COMPLETE_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(&(), &COMPLETE_WAKER_VTABLE),
    |_| {},
    |_| {},
    |_| {},
);

// SAFETY: This waker does nothing.
static COMPLETE_WAKER: Waker =
    unsafe { Waker::from_raw(RawWaker::new(&(), &COMPLETE_WAKER_VTABLE)) };

/// The state of an execution. We keep track of this because it's important to
/// correctly interact with functions that yield (like generators and streams)
/// by initially just calling the function, then by providing a value pushed
/// onto the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExecutionState {
    /// The initial state of an execution.
    Initial,
    /// execution is waiting.
    Resumed(Output),
    /// Suspended execution.
    Suspended,
    /// Execution exited.
    Exited(Option<InstAddress>),
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionState::Initial => write!(f, "initial"),
            ExecutionState::Resumed(out) => write!(f, "resumed({out})"),
            ExecutionState::Suspended => write!(f, "suspended"),
            ExecutionState::Exited(..) => write!(f, "exited"),
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
    state: ExecutionState,
    /// Indicates the current stack of suspended contexts.
    states: Vec<VmExecutionState>,
}

impl<T> VmExecution<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// Construct an execution from a virtual machine.
    pub(crate) fn new(head: T) -> Self {
        Self {
            head,
            state: ExecutionState::Initial,
            states: Vec::new(),
        }
    }

    /// Test if the current execution state is resumed.
    pub(crate) fn is_resumed(&self) -> bool {
        matches!(self.state, ExecutionState::Resumed(..))
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
    pub fn into_stream(self) -> Stream<T> {
        Stream::from_execution(self)
    }

    /// Get a reference to the current virtual machine.
    pub fn vm(&self) -> &Vm {
        self.head.as_ref()
    }

    /// Get a mutable reference the current virtual machine.
    pub fn vm_mut(&mut self) -> &mut Vm {
        self.head.as_mut()
    }

    /// Synchronously complete the current execution.
    ///
    /// # Errors
    ///
    /// If anything except the completion of the execution is encountered, this
    /// will result in an error.
    ///
    /// To handle other outcomes and more configurability see
    /// [`VmExecution::resume`].
    pub fn complete(&mut self) -> VmResult<Value> {
        vm_try!(self.resume().complete()).into_complete()
    }

    /// Asynchronously complete the current execution.
    ///
    /// # Errors
    ///
    /// If anything except the completion of the execution is encountered, this
    /// will result in an error.
    ///
    /// To handle other outcomes and more configurability see
    /// [`VmExecution::resume`].
    pub async fn async_complete(&mut self) -> VmResult<Value> {
        vm_try!(self.resume().await).into_complete()
    }

    /// Resume the current execution.
    ///
    /// To complete this operation synchronously, use [`VmResume::complete`].
    ///
    /// ## Resume with a value
    ///
    /// To resume an execution with a value, use [`VmResume::with_value`]. This
    /// requires that the execution has yielded first, otherwise an error will
    /// be produced.
    ///
    /// ## Resume with diagnostics
    ///
    /// To associated [`VmDiagnostics`] with the execution, use
    /// [`VmResume::with_diagnostics`].
    pub fn resume(&mut self) -> VmResume<'_, 'static, T> {
        VmResume {
            execution: self,
            diagnostics: None,
            awaited: None,
            init: Some(Init::Empty),
        }
    }

    /// Perform a single step of the execution.
    ///
    /// This will set the execution budget to `1`, which means that this
    /// execution can produce [`VmOutcome::Limited`].
    ///
    /// To complete this operation synchronously, use [`Step::complete`].
    pub fn step(&mut self) -> Step<'_, 'static, T> {
        Step {
            _budget: budget::replace(1),
            resume: VmResume {
                execution: self,
                diagnostics: None,
                awaited: None,
                init: None,
            },
        }
    }

    /// End execution and perform debug checks.
    pub(crate) fn end(&mut self) -> VmResult<Value> {
        let ExecutionState::Exited(addr) = self.state else {
            return VmResult::err(VmErrorKind::ExpectedExitedExecutionState { actual: self.state });
        };

        let value = match addr {
            Some(addr) => self.head.as_ref().stack().at(addr).clone(),
            None => Value::unit(),
        };

        debug_assert!(self.states.is_empty(), "Execution states should be empty");
        VmResult::Ok(value)
    }

    /// Push a virtual machine state onto the execution.
    #[tracing::instrument(skip_all)]
    pub(crate) fn push_state(&mut self, state: VmExecutionState) -> VmResult<()> {
        tracing::trace!("pushing suspended state");
        let vm = self.head.as_mut();
        let context = state.context.map(|c| replace(vm.context_mut(), c));
        let unit = state.unit.map(|u| replace(vm.unit_mut(), u));
        vm_try!(self.states.try_push(VmExecutionState { context, unit }));
        VmResult::Ok(())
    }

    /// Pop a virtual machine state from the execution and transfer the top of
    /// the stack from the popped machine.
    #[tracing::instrument(skip_all)]
    fn pop_state(&mut self) -> VmResult<()> {
        tracing::trace!("popping suspended state");

        let state = vm_try!(self.states.pop().ok_or(VmErrorKind::NoRunningVm));
        let vm = self.head.as_mut();

        if let Some(context) = state.context {
            *vm.context_mut() = context;
        }

        if let Some(unit) = state.unit {
            *vm.unit_mut() = unit;
        }

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
    pub fn complete(mut self) -> impl Future<Output = VmResult<Value>> + Send + 'static {
        let future = async move { self.0.resume().await.and_then(VmOutcome::into_complete) };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { AssertSend::new(future) }
    }

    /// Alias for [`VmSendExecution::complete`].
    #[deprecated = "Use `VmSendExecution::complete`"]
    pub fn async_complete(self) -> impl Future<Output = VmResult<Value>> + Send + 'static {
        self.complete()
    }

    /// Complete the current execution with support for async instructions.
    ///
    /// This requires that the result of the Vm is converted into a
    /// [crate::FromValue] that also implements [Send],  which prevents non-Send
    /// values from escaping from the virtual machine.
    pub fn complete_with_diagnostics(
        mut self,
        diagnostics: &mut dyn VmDiagnostics,
    ) -> impl Future<Output = VmResult<Value>> + Send + '_ {
        let future = async move {
            self.0
                .resume()
                .with_diagnostics(diagnostics)
                .await
                .and_then(VmOutcome::into_complete)
        };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { AssertSend::new(future) }
    }

    /// Alias for [`VmSendExecution::complete_with_diagnostics`].
    #[deprecated = "Use `VmSendExecution::complete_with_diagnostics`"]
    pub fn async_complete_with_diagnostics(
        self,
        diagnostics: &mut dyn VmDiagnostics,
    ) -> impl Future<Output = VmResult<Value>> + Send + '_ {
        self.complete_with_diagnostics(diagnostics)
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

/// Future that completes an execution returning the completed value.
///
/// This will error if the underlying execution produces a state which does not complete to a value.
pub struct AsyncComplete<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    future: VmResume<'this, 'diag, T>,
}

impl<'this, 'diag, T> Future for AsyncComplete<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    type Output = VmResult<Value>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = unsafe { Pin::map_unchecked_mut(self, |this| &mut this.future) };
        Poll::Ready(async_vm_try!(ready!(future.poll(cx))).into_complete())
    }
}

enum Init {
    Empty,
    Value(Value),
}

/// The outcome of completing an execution through a [`VmResume`] operation.
#[non_exhaustive]
pub enum VmOutcome {
    /// A value has been produced by the execution returning.
    Complete(Value),
    /// A value has been yielded by the execution.
    Yielded(Value),
    /// The execution has been limited.
    Limited,
}

impl VmOutcome {
    /// Convert the outcome into a [`GeneratorState`].
    ///
    /// # Errors
    ///
    /// If the execution is not in a state compatible with producing a generator
    /// state, such as having been completed or yielded, this will produce an
    /// error.
    pub fn into_generator_state(self) -> VmResult<GeneratorState> {
        match self {
            VmOutcome::Complete(value) => VmResult::Ok(GeneratorState::Complete(value)),
            VmOutcome::Yielded(value) => VmResult::Ok(GeneratorState::Yielded(value)),
            VmOutcome::Limited => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            }),
        }
    }

    /// Convert the outcome into a completed value.
    ///
    /// # Errors
    ///
    /// If the execution hasn't returned, this will produce an error.
    pub fn into_complete(self) -> VmResult<Value> {
        match self {
            VmOutcome::Complete(value) => VmResult::Ok(value),
            VmOutcome::Yielded(..) => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            }),
            VmOutcome::Limited => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            }),
        }
    }
}

/// An execution that has been resumed.
///
/// This can either be completed as a future, which allows the execution to
/// perform asynchronous operations, or it can be completed by calling
/// [`VmResume::complete`] which will produce an error in case asynchronous
/// operations that need to be suspended are encountered.
pub struct VmResume<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    execution: &'this mut VmExecution<T>,
    diagnostics: Option<&'diag mut dyn VmDiagnostics>,
    init: Option<Init>,
    awaited: Option<Awaited>,
}

impl<'this, 'diag, T> VmResume<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// Associated a budget with the resumed execution.
    pub fn with_budget(self, budget: usize) -> Budget<Self> {
        budget::with(budget, self)
    }

    /// Associate a value with the resumed execution.
    ///
    /// This is necessary to provide a value for a generator which has yielded.
    pub fn with_value(self, value: Value) -> VmResume<'this, 'diag, T> {
        Self {
            init: Some(Init::Value(value)),
            ..self
        }
    }

    /// Associate [`VmDiagnostics`] with the execution.
    pub fn with_diagnostics<'a>(
        self,
        diagnostics: &'a mut dyn VmDiagnostics,
    ) -> VmResume<'this, 'a, T> {
        VmResume {
            execution: self.execution,
            diagnostics: Some(diagnostics),
            init: self.init,
            awaited: self.awaited,
        }
    }

    /// Try to synchronously complete the run, returning the generator state it produced.
    ///
    /// This will error if the execution is suspended through awaiting.
    pub fn complete(self) -> VmResult<VmOutcome> {
        let this = pin!(self);
        let mut cx = Context::from_waker(&COMPLETE_WAKER);

        match this.poll(&mut cx) {
            Poll::Ready(result) => result,
            Poll::Pending => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Awaited,
            }),
        }
    }
}

impl<'this, 'diag, T> Future for VmResume<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    type Output = VmResult<VmOutcome>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We are ensuring that we never move this value or any
        // projected fields.
        let this = unsafe { Pin::get_unchecked_mut(self) };

        if let Some(init) = this.init.take() {
            let vm = this.execution.head.as_mut();

            match init {
                Init::Empty => {
                    if let ExecutionState::Resumed(out) =
                        replace(&mut this.execution.state, ExecutionState::Suspended)
                    {
                        async_vm_try!(out.store(vm.stack_mut(), Value::unit));
                    }
                }
                Init::Value(value) => {
                    let state = replace(&mut this.execution.state, ExecutionState::Suspended);

                    let ExecutionState::Resumed(out) = state else {
                        return Poll::Ready(VmResult::err(VmErrorKind::ExpectedExecutionState {
                            actual: state,
                        }));
                    };

                    async_vm_try!(out.store(vm.stack_mut(), value));
                }
            }
        }

        loop {
            let vm = this.execution.head.as_mut();

            if let Some(awaited) = &mut this.awaited {
                let awaited = unsafe { Pin::new_unchecked(awaited) };
                async_vm_try!(ready!(awaited.poll(cx, vm)));
                this.awaited = None;
            }

            match async_vm_try!(vm
                .run(match this.diagnostics {
                    Some(ref mut value) => Some(&mut **value),
                    None => None,
                })
                .with_vm(vm))
            {
                VmHalt::Exited(addr) => {
                    this.execution.state = ExecutionState::Exited(addr);
                }
                VmHalt::Awaited(awaited) => {
                    this.awaited = Some(awaited);
                    continue;
                }
                VmHalt::VmCall(vm_call) => {
                    async_vm_try!(vm_call.into_execution(this.execution));
                    continue;
                }
                VmHalt::Yielded(addr, out) => {
                    let value = match addr {
                        Some(addr) => vm.stack().at(addr).clone(),
                        None => Value::unit(),
                    };

                    this.execution.state = ExecutionState::Resumed(out);
                    return Poll::Ready(VmResult::Ok(VmOutcome::Yielded(value)));
                }
                VmHalt::Limited => {
                    return Poll::Ready(VmResult::Ok(VmOutcome::Limited));
                }
            }

            if this.execution.states.is_empty() {
                let value = async_vm_try!(this.execution.end());
                return Poll::Ready(VmResult::Ok(VmOutcome::Complete(value)));
            }

            async_vm_try!(this.execution.pop_state());
        }
    }
}

/// A future that governs a single step of an execution.
pub struct Step<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    _budget: budget::BudgetGuard,
    resume: VmResume<'this, 'diag, T>,
}

impl<'this, 'diag, T> Step<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// Try to synchronously complete the run, returning the generator state it
    /// produced.
    ///
    /// This will error if the execution is suspended through awaiting.
    #[inline]
    pub fn complete(self) -> VmResult<VmOutcome> {
        let this = pin!(self);
        let mut cx = Context::from_waker(&COMPLETE_WAKER);

        match this.poll(&mut cx) {
            Poll::Ready(result) => result,
            Poll::Pending => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Awaited,
            }),
        }
    }
}

impl<'this, 'diag, T> Future for Step<'this, 'diag, T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    type Output = VmResult<VmOutcome>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let resume = unsafe { Pin::map_unchecked_mut(self, |this| &mut this.resume) };
        resume.poll(cx)
    }
}
