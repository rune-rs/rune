use core::fmt;
use core::future::Future;
use core::mem::{replace, take};

use ::rust_alloc::sync::Arc;

use crate::alloc::Vec;
use crate::runtime::budget;
use crate::runtime::{
    Generator, GeneratorState, RuntimeContext, Stream, Unit, Value, Vm, VmErrorKind, VmHalt,
    VmHaltInfo, VmResult,
};
use crate::shared::AssertSend;

/// The state of an execution. We keep track of this because it's important to
/// correctly interact with functions that yield (like generators and streams)
/// by initially just calling the function, then by providing a value pushed
/// onto the stack.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ExecutionState {
    /// The initial state of an execution.
    Initial,
    /// The resumed state of an execution. This expects a value to be pushed
    /// onto the virtual machine before it is continued.
    Resumed,
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionState::Initial => write!(f, "initial"),
            ExecutionState::Resumed => write!(f, "resumed"),
        }
    }
}

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
        matches!(self.state, ExecutionState::Resumed)
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

    /// Complete the current execution without support for async instructions.
    ///
    /// This will error if the execution is suspended through yielding.
    pub async fn async_complete(&mut self) -> VmResult<Value> {
        match vm_try!(self.async_resume().await) {
            GeneratorState::Complete(value) => VmResult::Ok(value),
            GeneratorState::Yielded(..) => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            }),
        }
    }

    /// Complete the current execution without support for async instructions.
    ///
    /// If any async instructions are encountered, this will error. This will
    /// also error if the execution is suspended through yielding.
    pub fn complete(&mut self) -> VmResult<Value> {
        match vm_try!(self.resume()) {
            GeneratorState::Complete(value) => VmResult::Ok(value),
            GeneratorState::Yielded(..) => VmResult::err(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            }),
        }
    }

    /// Resume the current execution with the given value and resume
    /// asynchronous execution.
    pub async fn async_resume_with(&mut self, value: Value) -> VmResult<GeneratorState> {
        if !matches!(self.state, ExecutionState::Resumed) {
            return VmResult::err(VmErrorKind::ExpectedExecutionState {
                expected: ExecutionState::Resumed,
                actual: self.state,
            });
        }

        vm_try!(self.head.as_mut().stack_mut().push(value));
        self.inner_async_resume().await
    }

    /// Resume the current execution with support for async instructions.
    ///
    /// If the function being executed is a generator or stream this will resume
    /// it while returning a unit from the current `yield`.
    pub async fn async_resume(&mut self) -> VmResult<GeneratorState> {
        if matches!(self.state, ExecutionState::Resumed) {
            vm_try!(self.head.as_mut().stack_mut().push(Value::EmptyTuple));
        } else {
            self.state = ExecutionState::Resumed;
        }

        self.inner_async_resume().await
    }

    async fn inner_async_resume(&mut self) -> VmResult<GeneratorState> {
        loop {
            let vm = self.head.as_mut();

            match vm_try!(vm.run().with_vm(vm)) {
                VmHalt::Exited => (),
                VmHalt::Awaited(awaited) => {
                    vm_try!(awaited.into_vm(vm).await);
                    continue;
                }
                VmHalt::VmCall(vm_call) => {
                    vm_try!(vm_call.into_execution(self));
                    continue;
                }
                VmHalt::Yielded => {
                    let value = vm_try!(vm.stack_mut().pop());
                    return VmResult::Ok(GeneratorState::Yielded(value));
                }
                halt => {
                    return VmResult::err(VmErrorKind::Halted {
                        halt: halt.into_info(),
                    })
                }
            }

            if self.states.is_empty() {
                let value = vm_try!(self.end());
                return VmResult::Ok(GeneratorState::Complete(value));
            }

            vm_try!(self.pop_state());
        }
    }

    /// Resume the current execution with the given value and resume synchronous
    /// execution.
    #[tracing::instrument(skip_all, fields(?value))]
    pub fn resume_with(&mut self, value: Value) -> VmResult<GeneratorState> {
        if !matches!(self.state, ExecutionState::Resumed) {
            return VmResult::err(VmErrorKind::ExpectedExecutionState {
                expected: ExecutionState::Resumed,
                actual: self.state,
            });
        }

        vm_try!(self.head.as_mut().stack_mut().push(value));
        self.inner_resume()
    }

    /// Resume the current execution without support for async instructions.
    ///
    /// If the function being executed is a generator or stream this will resume
    /// it while returning a unit from the current `yield`.
    ///
    /// If any async instructions are encountered, this will error.
    #[tracing::instrument(skip_all)]
    pub fn resume(&mut self) -> VmResult<GeneratorState> {
        if matches!(self.state, ExecutionState::Resumed) {
            vm_try!(self.head.as_mut().stack_mut().push(Value::EmptyTuple));
        } else {
            self.state = ExecutionState::Resumed;
        }

        self.inner_resume()
    }

    fn inner_resume(&mut self) -> VmResult<GeneratorState> {
        loop {
            let len = self.states.len();
            let vm = self.head.as_mut();

            match vm_try!(vm.run().with_vm(vm)) {
                VmHalt::Exited => (),
                VmHalt::VmCall(vm_call) => {
                    vm_try!(vm_call.into_execution(self));
                    continue;
                }
                VmHalt::Yielded => {
                    let value = vm_try!(vm.stack_mut().pop());
                    return VmResult::Ok(GeneratorState::Yielded(value));
                }
                halt => {
                    return VmResult::err(VmErrorKind::Halted {
                        halt: halt.into_info(),
                    });
                }
            }

            if len == 0 {
                let value = vm_try!(self.end());
                return VmResult::Ok(GeneratorState::Complete(value));
            }

            vm_try!(self.pop_state());
        }
    }

    /// Step the single execution for one step without support for async
    /// instructions.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn step(&mut self) -> VmResult<Option<Value>> {
        let len = self.states.len();
        let vm = self.head.as_mut();

        match vm_try!(budget::with(1, || vm.run().with_vm(vm)).call()) {
            VmHalt::Exited => (),
            VmHalt::VmCall(vm_call) => {
                vm_try!(vm_call.into_execution(self));
                return VmResult::Ok(None);
            }
            VmHalt::Limited => return VmResult::Ok(None),
            halt => {
                return VmResult::err(VmErrorKind::Halted {
                    halt: halt.into_info(),
                })
            }
        }

        if len == 0 {
            let value = vm_try!(self.end());
            return VmResult::Ok(Some(value));
        }

        vm_try!(self.pop_state());
        VmResult::Ok(None)
    }

    /// Step the single execution for one step with support for async
    /// instructions.
    pub async fn async_step(&mut self) -> VmResult<Option<Value>> {
        let vm = self.head.as_mut();

        match vm_try!(budget::with(1, || vm.run().with_vm(vm)).call()) {
            VmHalt::Exited => (),
            VmHalt::Awaited(awaited) => {
                vm_try!(awaited.into_vm(vm).await);
                return VmResult::Ok(None);
            }
            VmHalt::VmCall(vm_call) => {
                vm_try!(vm_call.into_execution(self));
                return VmResult::Ok(None);
            }
            VmHalt::Limited => return VmResult::Ok(None),
            halt => {
                return VmResult::err(VmErrorKind::Halted {
                    halt: halt.into_info(),
                });
            }
        }

        if self.states.is_empty() {
            let value = vm_try!(self.end());
            return VmResult::Ok(Some(value));
        }

        vm_try!(self.pop_state());
        VmResult::Ok(None)
    }

    /// End execution and perform debug checks.
    pub(crate) fn end(&mut self) -> VmResult<Value> {
        let vm = self.head.as_mut();
        let value = vm_try!(vm.stack_mut().pop());
        debug_assert!(self.states.is_empty(), "execution vms should be empty");
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
    pub fn async_complete(mut self) -> impl Future<Output = VmResult<Value>> + Send + 'static {
        let future = async move {
            let result = vm_try!(self.0.async_resume().await);

            match result {
                GeneratorState::Complete(value) => VmResult::Ok(value),
                GeneratorState::Yielded(..) => VmResult::err(VmErrorKind::Halted {
                    halt: VmHaltInfo::Yielded,
                }),
            }
        };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { AssertSend::new(future) }
    }
}
