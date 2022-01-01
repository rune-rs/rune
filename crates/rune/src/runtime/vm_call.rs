use crate::runtime::{Call, Future, Generator, Stream, Value, Vm, VmError, VmExecution};

/// An instruction to push a virtual machine to the execution.
#[derive(Debug)]
pub(crate) struct VmCall {
    call: Call,
    vm: Vm,
}

impl VmCall {
    /// Construct a new nested vm call.
    pub(crate) fn new(call: Call, vm: Vm) -> Self {
        Self { call, vm }
    }

    /// Encode the push itno an execution.
    pub(crate) fn into_execution<T>(self, execution: &mut VmExecution<T>) -> Result<(), VmError>
    where
        T: AsMut<Vm>,
    {
        let mut vm = self.vm;

        let value = match self.call {
            Call::Async => Value::from(Future::new(async move { vm.async_complete().await })),
            Call::Immediate => {
                execution.push_vm(vm);
                return Ok(());
            }
            Call::Stream => Value::from(Stream::new(vm)),
            Call::Generator => Value::from(Generator::new(vm)),
        };

        let vm = execution.vm_mut();
        vm.stack_mut().push(value);
        vm.advance();
        Ok(())
    }
}
