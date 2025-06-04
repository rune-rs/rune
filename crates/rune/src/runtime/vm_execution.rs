use core::fmt;
use core::future::Future;
use core::mem::{replace, take};
use core::pin::{pin, Pin};
use core::task::{ready, Context, Poll, RawWaker, RawWakerVTable, Waker};

use rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::async_vm_try;
use crate::runtime::budget::Budget;
use crate::runtime::{budget, Awaited};
use crate::shared::AssertSend;

use super::{
    GeneratorState, InstAddress, Output, RuntimeContext, Unit, Value, Vm, VmDiagnostics, VmError,
    VmErrorKind, VmHalt, VmHaltInfo,
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
pub struct VmExecution<T> {
    /// The current head vm which holds the execution.
    vm: T,
    /// The state of an execution.
    state: ExecutionState,
    /// Indicates the current stack of suspended contexts.
    states: Vec<VmExecutionState>,
}

impl<T> VmExecution<T> {
    /// Get a reference to the current virtual machine.
    pub fn vm(&self) -> &Vm
    where
        T: AsRef<Vm>,
    {
        self.vm.as_ref()
    }

    /// Get a mutable reference the current virtual machine.
    pub fn vm_mut(&mut self) -> &mut Vm
    where
        T: AsMut<Vm>,
    {
        self.vm.as_mut()
    }

    /// Construct an execution from a virtual machine.
    pub(crate) fn new(vm: T) -> Self {
        Self {
            vm,
            state: ExecutionState::Initial,
            states: Vec::new(),
        }
    }
}

impl<T> VmExecution<T>
where
    T: AsMut<Vm>,
{
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
    /// while let Some(value) = generator.next()? {
    ///     let value: i64 = rune::from_value(value)?;
    ///     assert_eq!(value, n);
    ///     n += 1;
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn into_generator(self) -> VmGenerator<T> {
        VmGenerator {
            execution: Some(self),
        }
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
    /// while let Some(value) = stream.next().await? {
    ///     let value: i64 = rune::from_value(value)?;
    ///     assert_eq!(value, n);
    ///     n += 1;
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// # })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn into_stream(self) -> VmStream<T> {
        VmStream {
            execution: Some(self),
        }
    }
}

impl<T> VmExecution<T>
where
    T: AsMut<Vm>,
{
    /// Synchronously complete the current execution.
    ///
    /// # Errors
    ///
    /// If anything except the completion of the execution is encountered, this
    /// will result in an error.
    ///
    /// To handle other outcomes and more configurability see
    /// [`VmExecution::resume`].
    pub fn complete(&mut self) -> Result<Value, VmError> {
        self.resume().complete()?.into_complete()
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
    pub async fn async_complete(&mut self) -> Result<Value, VmError> {
        self.resume().await?.into_complete()
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
            init: Some(Value::empty()),
        }
    }

    /// End execution and perform debug checks.
    pub(crate) fn end(&mut self) -> Result<Value, VmError> {
        let ExecutionState::Exited(addr) = self.state else {
            return Err(VmError::new(VmErrorKind::ExpectedExitedExecutionState {
                actual: self.state,
            }));
        };

        let value = match addr {
            Some(addr) => self.vm.as_mut().stack().at(addr).clone(),
            None => Value::unit(),
        };

        debug_assert!(self.states.is_empty(), "Execution states should be empty");
        Ok(value)
    }

    /// Push a virtual machine state onto the execution.
    #[tracing::instrument(skip_all)]
    pub(crate) fn push_state(&mut self, state: VmExecutionState) -> Result<(), VmError> {
        tracing::trace!("pushing suspended state");
        let vm = self.vm.as_mut();
        let context = state.context.map(|c| replace(vm.context_mut(), c));
        let unit = state.unit.map(|u| replace(vm.unit_mut(), u));
        self.states.try_push(VmExecutionState { context, unit })?;
        Ok(())
    }

    /// Pop a virtual machine state from the execution and transfer the top of
    /// the stack from the popped machine.
    #[tracing::instrument(skip_all)]
    fn pop_state(&mut self) -> Result<(), VmError> {
        tracing::trace!("popping suspended state");

        let state = self.states.pop().ok_or(VmErrorKind::NoRunningVm)?;
        let vm = self.vm.as_mut();

        if let Some(context) = state.context {
            *vm.context_mut() = context;
        }

        if let Some(unit) = state.unit {
            *vm.unit_mut() = unit;
        }

        Ok(())
    }
}

impl VmExecution<&mut Vm> {
    /// Convert the current execution into one which owns its virtual machine.
    pub fn into_owned(self) -> VmExecution<Vm> {
        let stack = take(self.vm.stack_mut());
        let head = Vm::with_stack(self.vm.context().clone(), self.vm.unit().clone(), stack);

        VmExecution {
            vm: head,
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
    pub fn complete(mut self) -> impl Future<Output = Result<Value, VmError>> + Send + 'static {
        let future = async move { self.0.resume().await.and_then(VmOutcome::into_complete) };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { AssertSend::new(future) }
    }

    /// Alias for [`VmSendExecution::complete`].
    #[deprecated = "Use `VmSendExecution::complete`"]
    pub fn async_complete(self) -> impl Future<Output = Result<Value, VmError>> + Send + 'static {
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
    ) -> impl Future<Output = Result<Value, VmError>> + Send + '_ {
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
    ) -> impl Future<Output = Result<Value, VmError>> + Send + '_ {
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
            vm: self.vm.try_clone()?,
            state: self.state,
            states: self.states.try_clone()?,
        })
    }
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
    pub fn into_generator_state(self) -> Result<GeneratorState, VmError> {
        match self {
            VmOutcome::Complete(value) => Ok(GeneratorState::Complete(value)),
            VmOutcome::Yielded(value) => Ok(GeneratorState::Yielded(value)),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }

    /// Convert the outcome into a completed value.
    ///
    /// # Errors
    ///
    /// If the execution hasn't returned, this will produce an error.
    pub fn into_complete(self) -> Result<Value, VmError> {
        match self {
            VmOutcome::Complete(value) => Ok(value),
            VmOutcome::Yielded(..) => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            })),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }
}

/// An execution that has been resumed.
///
/// This can either be completed as a future, which allows the execution to
/// perform asynchronous operations, or it can be completed by calling
/// [`VmResume::complete`] which will produce an error in case asynchronous
/// operations that need to be suspended are encountered.
pub struct VmResume<'this, 'diag, T> {
    execution: &'this mut VmExecution<T>,
    diagnostics: Option<&'diag mut dyn VmDiagnostics>,
    init: Option<Value>,
    awaited: Option<Awaited>,
}

impl<'this, 'diag, T> VmResume<'this, 'diag, T> {
    /// Associated a budget with the resumed execution.
    pub fn with_budget(self, budget: usize) -> Budget<Self> {
        budget::with(budget, self)
    }

    /// Associate a value with the resumed execution.
    ///
    /// This is necessary to provide a value for a generator which has yielded.
    pub fn with_value(self, value: Value) -> VmResume<'this, 'diag, T> {
        Self {
            init: Some(value),
            ..self
        }
    }

    /// Associate diagnostics with the execution.
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
}

impl<'this, 'diag, T> VmResume<'this, 'diag, T>
where
    T: AsMut<Vm>,
{
    /// Try to synchronously complete the run, returning the generator state it produced.
    ///
    /// This will error if the execution is suspended through awaiting.
    pub fn complete(self) -> Result<VmOutcome, VmError> {
        let this = pin!(self);
        let mut cx = Context::from_waker(&COMPLETE_WAKER);

        match this.poll(&mut cx) {
            Poll::Ready(result) => result,
            Poll::Pending => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Awaited,
            })),
        }
    }
}

impl<'this, 'diag, T> Future for VmResume<'this, 'diag, T>
where
    T: AsMut<Vm>,
{
    type Output = Result<VmOutcome, VmError>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We are ensuring that we never move this value or any
        // projected fields.
        let this = unsafe { Pin::get_unchecked_mut(self) };

        if let Some(value) = this.init.take() {
            let state = replace(&mut this.execution.state, ExecutionState::Suspended);

            if let ExecutionState::Resumed(out) = state {
                let vm = this.execution.vm.as_mut();
                async_vm_try!(out.store(vm.stack_mut(), value));
            }
        }

        loop {
            let vm = this.execution.vm.as_mut();

            if let Some(awaited) = &mut this.awaited {
                let awaited = unsafe { Pin::new_unchecked(awaited) };
                async_vm_try!(ready!(awaited.poll(cx, vm)));
                this.awaited = None;
            }

            let result = vm.run(match this.diagnostics {
                Some(ref mut value) => Some(&mut **value),
                None => None,
            });

            match async_vm_try!(VmError::with_vm(result, vm)) {
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
                    return Poll::Ready(Ok(VmOutcome::Yielded(value)));
                }
                VmHalt::Limited => {
                    return Poll::Ready(Ok(VmOutcome::Limited));
                }
            }

            if this.execution.states.is_empty() {
                let value = async_vm_try!(this.execution.end());
                return Poll::Ready(Ok(VmOutcome::Complete(value)));
            }

            async_vm_try!(this.execution.pop_state());
        }
    }
}

/// A [`VmExecution`] that can be used with a generator api.
pub struct VmGenerator<T> {
    execution: Option<VmExecution<T>>,
}

impl<T> VmGenerator<T>
where
    T: AsMut<Vm>,
{
    /// Get the next value produced by this generator.
    ///
    /// See [`VmExecution::into_generator`].
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        let Some(execution) = &mut self.execution else {
            return Ok(None);
        };

        let outcome = execution.resume().complete()?;

        match outcome {
            VmOutcome::Complete(_) => {
                self.execution = None;
                Ok(None)
            }
            VmOutcome::Yielded(value) => Ok(Some(value)),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }

    /// Resume the generator with a value and get the next [`GeneratorState`].
    ///
    /// See [`VmExecution::into_generator`].
    pub fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete)?;

        let outcome = execution.resume().with_value(value).complete()?;

        match outcome {
            VmOutcome::Complete(value) => {
                self.execution = None;
                Ok(GeneratorState::Complete(value))
            }
            VmOutcome::Yielded(value) => Ok(GeneratorState::Yielded(value)),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }
}

/// A [`VmExecution`] that can be used with a stream api.
pub struct VmStream<T> {
    execution: Option<VmExecution<T>>,
}

impl<T> VmStream<T>
where
    T: AsMut<Vm>,
{
    /// Get the next value produced by this stream.
    ///
    /// See [`VmExecution::into_stream`].
    pub async fn next(&mut self) -> Result<Option<Value>, VmError> {
        let Some(execution) = &mut self.execution else {
            return Ok(None);
        };

        match execution.resume().await? {
            VmOutcome::Complete(value) => {
                self.execution = None;
                Ok(Some(value))
            }
            VmOutcome::Yielded(..) => Ok(None),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }

    /// Resume the stream with a value and return the next [`GeneratorState`].
    ///
    /// See [`VmExecution::into_stream`].
    pub async fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete)?;

        match execution.resume().with_value(value).await? {
            VmOutcome::Complete(value) => {
                self.execution = None;
                Ok(GeneratorState::Complete(value))
            }
            VmOutcome::Yielded(value) => Ok(GeneratorState::Yielded(value)),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }
}
