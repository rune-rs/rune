use core::future::Future as _;
use core::pin::Pin;
use core::task::{ready, Context, Poll};

use crate::async_vm_try;
use crate::runtime::{Future, Output, Select, Vm, VmError};

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
    ) -> Poll<Result<(), VmError>> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        match *this {
            Self::Future(ref mut future, out) => {
                let future = unsafe { Pin::new_unchecked(future) };
                let result = ready!(future.poll(cx));
                let value = async_vm_try!(VmError::with_vm(result, vm));
                async_vm_try!(vm.stack_mut().store(out, value));
            }
            Self::Select(ref mut select, out) => {
                let select = unsafe { Pin::new_unchecked(select) };
                let result = ready!(select.poll(cx));
                let (ip, value) = async_vm_try!(VmError::with_vm(result, vm));
                vm.set_ip(ip);
                async_vm_try!(vm.stack_mut().store(out, || value));
            }
        }

        Poll::Ready(Ok(()))
    }
}
