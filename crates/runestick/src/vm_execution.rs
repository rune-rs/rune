use crate::budget;
use crate::internal;
use crate::{GeneratorState, Value, Vm, VmError, VmErrorKind, VmHalt, VmHaltInfo};
use std::future::Future;

/// The execution environment for a virtual machine.
pub struct VmExecution {
    vms: Vec<Vm>,
}

impl VmExecution {
    /// Construct an execution from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self { vms: vec![vm] }
    }

    /// Get the current virtual machine.
    pub fn vm(&self) -> Result<&Vm, VmError> {
        match self.vms.last() {
            Some(vm) => Ok(vm),
            None => Err(VmError::from(VmErrorKind::NoRunningVm)),
        }
    }

    /// Get the current virtual machine mutably.
    pub fn vm_mut(&mut self) -> Result<&mut Vm, VmError> {
        match self.vms.last_mut() {
            Some(vm) => Ok(vm),
            None => Err(VmError::from(VmErrorKind::NoRunningVm)),
        }
    }

    /// Complete the current execution without support for async instructions.
    ///
    /// This will error if the execution is suspended through yielding.
    pub async fn async_complete(&mut self) -> Result<Value, VmError> {
        match self.async_resume().await? {
            GeneratorState::Complete(value) => Ok(value),
            GeneratorState::Yielded(..) => Err(VmError::from(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            })),
        }
    }

    /// Complete the current execution without support for async instructions.
    ///
    /// If any async instructions are encountered, this will error. This will
    /// also error if the execution is suspended through yielding.
    pub fn complete(&mut self) -> Result<Value, VmError> {
        match self.resume()? {
            GeneratorState::Complete(value) => Ok(value),
            GeneratorState::Yielded(..) => Err(VmError::from(VmErrorKind::Halted {
                halt: VmHaltInfo::Yielded,
            })),
        }
    }

    /// Resume the current execution with support for async instructions.
    pub async fn async_resume(&mut self) -> Result<GeneratorState, VmError> {
        loop {
            let len = self.vms.len();
            let vm = self.vm_mut()?;

            match Self::run(vm)? {
                VmHalt::Exited => (),
                VmHalt::Awaited(awaited) => {
                    awaited.into_vm(vm).await?;
                    continue;
                }
                VmHalt::VmCall(vm_call) => {
                    vm_call.into_execution(self)?;
                    continue;
                }
                VmHalt::Yielded => return Ok(GeneratorState::Yielded(vm.stack_mut().pop()?)),
                halt => {
                    return Err(VmError::from(VmErrorKind::Halted {
                        halt: halt.into_info(),
                    }))
                }
            }

            if len == 1 {
                let value = vm.stack_mut().pop()?;
                debug_assert!(vm.stack().is_empty(), "the final vm should be empty");
                self.vms.clear();
                return Ok(GeneratorState::Complete(value));
            }

            self.pop_vm()?;
        }
    }

    /// Resume the current execution without support for async instructions.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn resume(&mut self) -> Result<GeneratorState, VmError> {
        loop {
            let len = self.vms.len();
            let vm = self.vm_mut()?;

            match Self::run(vm)? {
                VmHalt::Exited => (),
                VmHalt::VmCall(vm_call) => {
                    vm_call.into_execution(self)?;
                    continue;
                }
                VmHalt::Yielded => return Ok(GeneratorState::Yielded(vm.stack_mut().pop()?)),
                halt => {
                    return Err(VmError::from(VmErrorKind::Halted {
                        halt: halt.into_info(),
                    }))
                }
            }

            if len == 1 {
                let value = vm.stack_mut().pop()?;
                debug_assert!(vm.stack().is_empty(), "the final vm should be empty");
                self.vms.clear();
                return Ok(GeneratorState::Complete(value));
            }

            self.pop_vm()?;
        }
    }

    /// Step the single execution for one step without support for async
    /// instructions.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn step(&mut self) -> Result<Option<Value>, VmError> {
        let len = self.vms.len();
        let vm = self.vm_mut()?;

        match budget::with(1, || Self::run(vm)).call()? {
            VmHalt::Exited => (),
            VmHalt::VmCall(vm_call) => {
                vm_call.into_execution(self)?;
                return Ok(None);
            }
            VmHalt::Limited => return Ok(None),
            halt => {
                return Err(VmError::from(VmErrorKind::Halted {
                    halt: halt.into_info(),
                }))
            }
        }

        if len == 1 {
            let value = vm.stack_mut().pop()?;
            debug_assert!(vm.stack().is_empty(), "final vm stack not clean");
            return Ok(Some(value));
        }

        self.pop_vm()?;
        Ok(None)
    }

    /// Step the single execution for one step with support for async
    /// instructions.
    pub async fn async_step(&mut self) -> Result<Option<Value>, VmError> {
        let len = self.vms.len();
        let vm = self.vm_mut()?;

        match budget::with(1, || Self::run(vm)).call()? {
            VmHalt::Exited => (),
            VmHalt::Awaited(awaited) => {
                awaited.into_vm(vm).await?;
                return Ok(None);
            }
            VmHalt::VmCall(vm_call) => {
                vm_call.into_execution(self)?;
                return Ok(None);
            }
            VmHalt::Limited => return Ok(None),
            halt => {
                return Err(VmError::from(VmErrorKind::Halted {
                    halt: halt.into_info(),
                }))
            }
        }

        if len == 1 {
            let value = vm.stack_mut().pop()?;
            debug_assert!(vm.stack().is_empty(), "final vm stack not clean");
            return Ok(Some(value));
        }

        self.pop_vm()?;
        Ok(None)
    }

    /// Push a virtual machine state onto the execution.
    pub(crate) fn push_vm(&mut self, vm: Vm) {
        self.vms.push(vm);
    }

    /// Pop a virtual machine state from the execution and transfer the top of
    /// the stack from the popped machine.
    fn pop_vm(&mut self) -> Result<(), VmError> {
        let mut from = self.vms.pop().ok_or(VmErrorKind::NoRunningVm)?;

        let stack = from.stack_mut();
        let value = stack.pop()?;
        debug_assert!(stack.is_empty(), "vm stack not clean");

        let onto = self.vm_mut()?;
        onto.stack_mut().push(value);
        onto.advance();
        Ok(())
    }

    #[inline]
    fn run(vm: &mut Vm) -> Result<VmHalt, VmError> {
        match vm.run() {
            Ok(reason) => Ok(reason),
            Err(error) => Err(error.into_unwinded(vm.unit(), vm.ip(), vm.call_frames().to_vec())),
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
pub struct VmSendExecution(pub(crate) VmExecution);

// Safety: we wrap all APIs around the [VmExecution], preventing values from
// escaping from contained virtual machine.
unsafe impl Send for VmSendExecution {}

impl VmSendExecution {
    /// Complete the current execution with support for async instructions.
    ///
    /// This requires that the result of the Vm is converted into a
    /// [crate::FromValue] that also implements [Send],  which prevents non-Send
    /// values from escaping from the virtual machine.
    pub fn async_complete(
        mut self,
    ) -> impl Future<Output = Result<Value, VmError>> + Send + 'static {
        let future = async move {
            let result = self.0.async_resume().await?;

            match result {
                GeneratorState::Complete(value) => Ok(value),
                GeneratorState::Yielded(..) => Err(VmError::from(VmErrorKind::Halted {
                    halt: VmHaltInfo::Yielded,
                })),
            }
        };

        // Safety: we wrap all APIs around the [VmExecution], preventing values
        // from escaping from contained virtual machine.
        unsafe { internal::AssertSend::new(future) }
    }
}
