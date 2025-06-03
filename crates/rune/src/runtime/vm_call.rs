use rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::runtime::vm_execution::VmExecutionState;
use crate::runtime::{
    Call, Future, Generator, Output, RuntimeContext, Stack, Stream, Unit, Value, Vm, VmError,
    VmErrorKind, VmExecution,
};

/// An instruction to push a virtual machine to the execution.
#[derive(Debug)]
#[must_use = "The construction of a vm call leaves the virtual machine stack in an intermediate state, VmCall::into_execution must be called to fix it"]
pub(crate) struct VmCall {
    /// The calling convention to use for the call.
    call: Call,
    /// Is set if the context differs for the call for the current virtual machine.
    context: Option<Arc<RuntimeContext>>,
    /// Is set if the unit differs for the call for the current virtual machine.
    unit: Option<Arc<Unit>>,
    /// The output to store the result of the call into.
    out: Output,
}

impl VmCall {
    pub(crate) fn new(
        call: Call,
        context: Option<Arc<RuntimeContext>>,
        unit: Option<Arc<Unit>>,
        out: Output,
    ) -> Self {
        Self {
            call,
            context,
            unit,
            out,
        }
    }

    /// Encode the push itno an execution.
    #[tracing::instrument(skip_all)]
    pub(crate) fn into_execution<T>(self, execution: &mut VmExecution<T>) -> Result<(), VmError>
    where
        T: AsMut<Vm>,
    {
        let out = self.out;

        let value = match self.call {
            Call::Async => {
                let vm = self.build_vm(execution)?;
                let mut execution = vm.into_execution();
                Value::try_from(Future::new(async move {
                    execution.resume().await?.into_complete()
                })?)?
            }
            Call::Immediate => {
                execution.push_state(VmExecutionState {
                    context: self.context,
                    unit: self.unit,
                })?;

                return Ok(());
            }
            Call::Stream => {
                let vm = self.build_vm(execution)?;
                Value::try_from(Stream::new(vm))?
            }
            Call::Generator => {
                let vm = self.build_vm(execution)?;
                Value::try_from(Generator::new(vm))?
            }
        };

        out.store(execution.vm_mut().stack_mut(), value)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn build_vm<T>(self, execution: &mut VmExecution<T>) -> Result<Vm, VmError>
    where
        T: AsMut<Vm>,
    {
        let vm = execution.vm_mut();

        let new_stack = vm.stack_mut().drain().try_collect::<Stack>()?;

        let Some(ip) = vm.pop_call_frame_from_call() else {
            return Err(VmError::new(VmErrorKind::MissingCallFrame));
        };

        let context = self.context.unwrap_or_else(|| vm.context().clone());
        let unit = self.unit.unwrap_or_else(|| vm.unit().clone());

        let mut vm = Vm::with_stack(context, unit, new_stack);
        vm.set_ip(ip);
        Ok(vm)
    }
}
