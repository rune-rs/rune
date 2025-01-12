use core::future::Future as _;
use core::pin::Pin;
use core::task::Poll;
use core::task::{ready, Context};

use crate::runtime::{Future, Output, Select, Vm, VmResult};

use super::future_vm_try;

/// A stored await task.
#[derive(Debug)]
pub(super) enum Awaited {
    /// A future to be awaited.
    Future(Future, Output),
    /// A select to be awaited.
    Select(Select, Output),
}

impl Awaited {
    /// Poll the inner thing being awaited.
    pub(super) fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        vm: &mut Vm,
    ) -> Poll<VmResult<()>> {
        let this = self.get_mut();

        match this {
            Awaited::Future(future, out) => {
                let future = Pin::new(future);
                let value = future_vm_try!(ready!(future.poll(cx)).with_vm(vm));
                future_vm_try!(out.store(vm.stack_mut(), value));
            }
            Awaited::Select(select, value_addr) => {
                let select = Pin::new(select);
                let (ip, value) = future_vm_try!(ready!(select.poll(cx)).with_vm(vm));
                vm.set_ip(ip);
                future_vm_try!(value_addr.store(vm.stack_mut(), || value));
            }
        }

        Poll::Ready(VmResult::Ok(()))
    }
}
