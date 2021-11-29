use crate::runtime::{Future, Generator, Stream, Value, Vm, VmError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// How the function is called.
///
/// Async functions create a sub-context and immediately return futures.
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
    /// A stream which has been resumed.
    ResumedStream,
    /// Function produces a generator.
    Generator,
    /// A generator that has been resumed.
    ResumedGenerator,
}

impl Call {
    /// Perform the call with the given virtual machine.
    #[inline]
    pub(crate) fn call_with_vm(self, vm: Vm) -> Result<Value, VmError> {
        Ok(match self {
            Call::Stream => Value::from(Stream::new(vm)),
            Call::Generator => Value::from(Generator::new(vm)),
            Call::ResumedGenerator | Call::Immediate => vm.complete()?,
            Call::ResumedStream | Call::Async => Value::from(Future::new(vm.async_complete())),
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
            Self::ResumedStream => {
                write!(fmt, "resumed stream")?;
            }
            Self::Generator => {
                write!(fmt, "generator")?;
            }
            Self::ResumedGenerator => {
                write!(fmt, "resumed generator")?;
            }
        }

        Ok(())
    }
}
