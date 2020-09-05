use crate::{Future, Select, Shared, ToValue, Vm, VmError};

/// A stored await task.
#[derive(Debug)]
pub enum Awaited {
    /// A future to be awaited.
    Future(Shared<Future>),
    /// A select to be awaited.
    Select(Select),
}

impl Awaited {
    /// Wait for the given awaited into the specified virtual machine.
    pub(crate) async fn into_vm(self, vm: &mut Vm) -> Result<(), VmError> {
        match self {
            Self::Future(future) => {
                let value = future.borrow_mut()?.await?;
                vm.stack_mut().push(value);
                vm.advance();
            }
            Self::Select(select) => {
                let (branch, value) = select.await?;
                vm.stack_mut().push(value);
                vm.stack_mut().push(ToValue::to_value(branch)?);
                vm.advance();
            }
        }

        Ok(())
    }
}
