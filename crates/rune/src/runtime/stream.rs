use crate::compile::{InstallWith, Named};
use crate::runtime::{
    FromValue, GeneratorState, Mut, RawMut, RawRef, RawStr, Ref, Shared, UnsafeFromValue, Value,
    Vm, VmError, VmErrorKind, VmExecution,
};
use std::fmt;

/// A stream with a stored virtual machine.
pub struct Stream<T>
where
    T: AsMut<Vm>,
{
    execution: Option<VmExecution<T>>,
}

impl<T> Stream<T>
where
    T: AsMut<Vm>,
{
    /// Construct a stream from a virtual machine.
    pub(crate) fn new(vm: T) -> Self {
        Self {
            execution: Some(VmExecution::new(vm)),
        }
    }

    /// Construct a generator from a complete execution.
    pub(crate) fn from_execution(execution: VmExecution<T>) -> Self {
        Self {
            execution: Some(execution),
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
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete)?;

        let state = if execution.is_resumed() {
            execution.async_resume_with(value).await?
        } else {
            execution.async_resume().await?
        };

        if state.is_complete() {
            self.execution = None;
        }

        Ok(state)
    }
}

impl Stream<&mut Vm> {
    /// Convert the current stream into one which owns its virtual machine.
    pub fn into_owned(self) -> Stream<Vm> {
        Stream {
            execution: self.execution.map(|e| e.into_owned()),
        }
    }
}

impl<T> Named for Stream<T>
where
    T: AsMut<Vm>,
{
    const BASE_NAME: RawStr = RawStr::from_str("Stream");
}

impl<T> InstallWith for Stream<T> where T: AsMut<Vm> {}

impl<T> fmt::Debug for Stream<T>
where
    T: AsMut<Vm>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl FromValue for Shared<Stream<Vm>> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_stream()
    }
}

impl FromValue for Stream<Vm> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let stream = value.into_stream()?;
        Ok(stream.take()?)
    }
}

impl UnsafeFromValue for &Stream<Vm> {
    type Output = *const Stream<Vm>;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let stream = value.into_stream()?;
        let (stream, guard) = Ref::into_raw(stream.into_ref()?);
        Ok((stream, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Stream<Vm> {
    type Output = *mut Stream<Vm>;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let stream = value.into_stream()?;
        Ok(Mut::into_raw(stream.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
