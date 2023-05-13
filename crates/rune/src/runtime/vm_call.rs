use crate::runtime::{Call, Future, Generator, Stream, Value, Vm, VmExecution, VmResult};

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
    pub(crate) fn into_execution<T>(self, execution: &mut VmExecution<T>) -> VmResult<()>
    where
        T: AsMut<Vm>,
    {
        let value = match self.call {
            Call::Async => {
                let mut execution = self.vm.into_execution();
                Value::from(Future::new(async move { execution.async_complete().await }))
            }
            Call::Immediate => {
                execution.push_vm(self.vm);
                return VmResult::Ok(());
            }
            Call::Stream => Value::from(Stream::new(self.vm)),
            Call::Generator => Value::from(Generator::new(self.vm)),
        };

        let vm = execution.vm_mut();
        vm.stack_mut().push(value);
        VmResult::Ok(())
    }
}
