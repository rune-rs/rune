use crate::{
    FromValue, GeneratorState, Mut, Named, RawMut, RawRef, RawStr, Ref, Shared, UnsafeFromValue,
    Value, Vm, VmError, VmErrorKind, VmExecution,
};
use std::fmt;
use std::mem;

/// A stream with a stored virtual machine.
pub struct Stream {
    execution: Option<VmExecution>,
    first: bool,
}

impl Stream {
    /// Construct a stream from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self {
            execution: Some(VmExecution::new(vm)),
            first: true,
        }
    }

    /// Get the next value produced by this stream.
    pub async fn next(&mut self) -> Result<Option<Value>, VmError> {
        Ok(match self.resume(Value::Unit).await? {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => None,
        })
    }

    /// Get the next value produced by this stream.
    pub async fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = match &mut self.execution {
            Some(execution) => execution,
            None => {
                return Err(VmError::from(VmErrorKind::GeneratorComplete));
            }
        };

        if !mem::take(&mut self.first) {
            execution.vm_mut()?.stack_mut().push(value);
        }

        let state = execution.async_resume().await?;

        if state.is_complete() {
            self.execution = None;
        }

        Ok(state)
    }
}

impl fmt::Debug for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl FromValue for Shared<Stream> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_stream()?)
    }
}

impl FromValue for Stream {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let stream = value.into_stream()?;
        Ok(stream.take()?)
    }
}

impl UnsafeFromValue for &Stream {
    type Output = *const Stream;
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let stream = value.into_stream()?;
        let (stream, guard) = Ref::into_raw(stream.into_ref()?);
        Ok((stream, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Stream {
    type Output = *mut Stream;
    type Guard = RawMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let stream = value.into_stream()?;
        Ok(Mut::into_raw(stream.into_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl Named for Stream {
    const NAME: RawStr = RawStr::from_str("Stream");
}
