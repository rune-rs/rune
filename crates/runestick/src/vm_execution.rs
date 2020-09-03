use crate::{StopReason, Value, Vm, VmError, VmErrorKind};

/// The execution environment for a virtual machine.
pub struct VmExecution {
    vms: Vec<Vm>,
}

impl VmExecution {
    /// Construct an execution from a virtual machine.
    pub fn of(vm: Vm) -> Self {
        Self { vms: vec![vm] }
    }

    /// Get the current virtual machine.
    pub fn vm(&self) -> Result<&Vm, VmError> {
        match self.vms.last() {
            Some(vm) => Ok(vm),
            None => {
                return Err(VmError::from(VmErrorKind::NoRunningVm));
            }
        }
    }

    /// Get the current virtual machine mutably.
    pub fn vm_mut(&mut self) -> Result<&mut Vm, VmError> {
        match self.vms.last_mut() {
            Some(vm) => Ok(vm),
            None => {
                return Err(VmError::from(VmErrorKind::NoRunningVm));
            }
        }
    }

    /// Run the given task to completion.
    pub async fn run_to_completion(&mut self) -> Result<Value, VmError> {
        loop {
            let len = self.vms.len();
            let vm = self.vm_mut()?;

            match vm.run_for(None).await? {
                StopReason::Exited => (),
                StopReason::Awaited(awaited) => {
                    // TODO: handle this through polling instead.
                    awaited.wait_with_vm(vm).await?;
                    continue;
                }
                StopReason::CallVm(call_vm) => {
                    call_vm.into_execution(self)?;
                    continue;
                }
                reason => {
                    return Err(VmError::from(VmErrorKind::Stopped {
                        reason: reason.into_info(),
                    }))
                }
            }

            if len == 1 {
                let value = vm.stack_mut().pop()?;
                debug_assert!(vm.stack().is_empty(), "the final vm should be empty");
                return Ok(value);
            }

            self.pop_vm()?;
        }
    }

    /// Run the execution for one step.
    pub async fn step(&mut self) -> Result<Option<Value>, VmError> {
        let len = self.vms.len();
        let vm = self.vm_mut()?;

        match vm.run_for(Some(1)).await? {
            StopReason::Exited => (),
            StopReason::Awaited(awaited) => {
                awaited.wait_with_vm(vm).await?;
                return Ok(None);
            }
            StopReason::CallVm(call_vm) => {
                call_vm.into_execution(self)?;
                return Ok(None);
            }
            StopReason::Limited => return Ok(None),
            reason => {
                return Err(VmError::from(VmErrorKind::Stopped {
                    reason: reason.into_info(),
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
        let mut from = self
            .vms
            .pop()
            .ok_or_else(|| VmError::from(VmErrorKind::NoRunningVm))?;

        let stack = from.stack_mut();
        let value = stack.pop()?;
        debug_assert!(stack.is_empty(), "vm stack not clean");

        let onto = self.vm_mut()?;
        onto.stack_mut().push(value);
        onto.advance();
        Ok(())
    }
}
