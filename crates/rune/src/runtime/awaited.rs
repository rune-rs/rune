use crate::runtime::{Future, Select, Shared, ToValue, Vm, VmResult};

/// A stored await task.
#[derive(Debug)]
pub(crate) enum Awaited {
    /// A future to be awaited.
    Future(Shared<Future>),
    /// A select to be awaited.
    Select(Select),
}

impl Awaited {
    /// Wait for the given awaited into the specified virtual machine.
    pub(crate) async fn into_vm(self, vm: &mut Vm) -> VmResult<()> {
        match self {
            Self::Future(future) => {
                let value = vm_try!(vm_try!(future.borrow_mut()).await.with_vm(vm));
                vm_try!(vm.stack_mut().push(value));
            }
            Self::Select(select) => {
                let (branch, value) = vm_try!(select.await.with_vm(vm));
                vm_try!(vm.stack_mut().push(value));
                vm_try!(vm.stack_mut().push(vm_try!(ToValue::to_value(branch))));
            }
        }

        VmResult::Ok(())
    }
}
