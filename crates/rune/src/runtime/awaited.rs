use core::future::Future as _;
use core::pin::Pin;
use core::task::{ready, Context, Poll};

use crate::async_vm_try;
use crate::runtime::{Future, Output, Select, Vm, VmResult};

/// A stored await task.
#[derive(Debug)]
pub(crate) enum Awaited {
    /// A future to be awaited.
    Future(Future, Output),
    /// A select to be awaited.
    Select(Select, Output),
}

impl Awaited {
    pub(crate) fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        vm: &mut Vm,
    ) -> Poll<VmResult<()>> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        match this {
            Self::Future(future, out) => {
                let future = unsafe { Pin::new_unchecked(future) };
                let result = ready!(future.poll(cx));
                let value = async_vm_try!(result.with_vm(vm));
                async_vm_try!(out.store(vm.stack_mut(), value));
            }
            Self::Select(select, value_addr) => {
                let select = unsafe { Pin::new_unchecked(select) };
                let result = ready!(select.poll(cx));
                let (ip, value) = async_vm_try!(result.with_vm(vm));
                vm.set_ip(ip);
                async_vm_try!(value_addr.store(vm.stack_mut(), || value));
            }
        }

        Poll::Ready(VmResult::Ok(()))
    }
}
