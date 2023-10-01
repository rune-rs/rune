use core::fmt;

use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::runtime::{Future, Generator, Stream, Value, Vm, VmResult};

/// The calling convention of a function.
#[derive(Debug, TryClone, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum Call {
    /// Function is `async` and returns a future that must be await:ed to make
    /// progress.
    Async,
    /// Functions are immediately called and control handed over.
    Immediate,
    /// Function produces a stream, also known as an async generator.
    Stream,
    /// Function produces a generator.
    Generator,
}

impl Call {
    /// Perform the call with the given virtual machine.
    #[inline]
    pub(crate) fn call_with_vm(self, vm: Vm) -> VmResult<Value> {
        VmResult::Ok(match self {
            Call::Stream => vm_try!(Value::try_from(Stream::new(vm))),
            Call::Generator => vm_try!(Value::try_from(Generator::new(vm))),
            Call::Immediate => vm_try!(vm.complete()),
            Call::Async => {
                let mut execution = vm.into_execution();
                let future = vm_try!(Future::new(async move { execution.async_complete().await }));
                vm_try!(Value::try_from(future))
            }
        })
    }
}

impl fmt::Display for Call {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immediate => {
                write!(fmt, "immediate")?;
            }
            Self::Async => {
                write!(fmt, "async")?;
            }
            Self::Stream => {
                write!(fmt, "stream")?;
            }
            Self::Generator => {
                write!(fmt, "generator")?;
            }
        }

        Ok(())
    }
}
