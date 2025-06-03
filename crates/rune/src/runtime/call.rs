use core::fmt;

#[cfg(feature = "musli")]
use musli::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::runtime::{Future, Generator, Stream, Value, Vm, VmError};

/// The calling convention of a function.
#[derive(Debug, TryClone, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Encode, Decode))]
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
    pub(crate) fn call_with_vm(self, vm: Vm) -> Result<Value, VmError> {
        Ok(match self {
            Call::Stream => Value::try_from(Stream::new(vm))?,
            Call::Generator => Value::try_from(Generator::new(vm))?,
            Call::Immediate => vm.complete()?,
            Call::Async => {
                let mut execution = vm.into_execution();
                let future = Future::new(async move { execution.resume().await?.into_complete() })?;
                Value::try_from(future)?
            }
        })
    }
}

impl fmt::Display for Call {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immediate => {
                write!(f, "immediate")
            }
            Self::Async => {
                write!(f, "async")
            }
            Self::Stream => {
                write!(f, "stream")
            }
            Self::Generator => {
                write!(f, "generator")
            }
        }
    }
}
