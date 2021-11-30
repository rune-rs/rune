use crate::runtime::{Future, Generator, Stream, Value, Vm, VmError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The calling convention of a function.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
            Call::Stream => Value::from(Stream::new(vm)),
            Call::Generator => Value::from(Generator::new(vm)),
            Call::Immediate => vm.complete()?,
            Call::Async => Value::from(Future::new(vm.async_complete())),
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
