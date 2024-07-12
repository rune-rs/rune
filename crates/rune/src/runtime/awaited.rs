use crate::runtime::{Future, Output, Select, Vm, VmResult};

/// A stored await task.
#[derive(Debug)]
pub(crate) enum Awaited {
    /// A future to be awaited.
    Future(Future, Output),
    /// A select to be awaited.
    Select(Select, Output, Output),
}

impl Awaited {
    /// Wait for the given awaited into the specified virtual machine.
    pub(crate) async fn into_vm(self, vm: &mut Vm) -> VmResult<()> {
        match self {
            Self::Future(future, out) => {
                let value = vm_try!(future.await.with_vm(vm));
                vm_try!(out.store(vm.stack_mut(), value));
            }
            Self::Select(select, branch_addr, value_addr) => {
                let (branch, value) = vm_try!(select.await.with_vm(vm));
                vm_try!(branch_addr.store(vm.stack_mut(), || branch));
                vm_try!(value_addr.store(vm.stack_mut(), || value));
            }
        }

        VmResult::Ok(())
    }
}
