use core::fmt;

use crate::compile::{InstallWith, Named};
use crate::runtime::{
    FromValue, GeneratorState, Mut, RawMut, RawRef, RawStr, Ref, Shared, UnsafeFromValue, Value,
    Vm, VmErrorKind, VmExecution, VmResult,
};

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
    pub async fn next(&mut self) -> VmResult<Option<Value>> {
        VmResult::Ok(match vm_try!(self.resume(Value::Unit).await) {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => None,
        })
    }

    /// Get the next value produced by this stream.
    pub async fn resume(&mut self, value: Value) -> VmResult<GeneratorState> {
        let execution = vm_try!(self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete));

        let state = if execution.is_resumed() {
            vm_try!(execution.async_resume_with(value).await)
        } else {
            vm_try!(execution.async_resume().await)
        };

        if state.is_complete() {
            self.execution = None;
        }

        VmResult::Ok(state)
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
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_stream()
    }
}

impl FromValue for Stream<Vm> {
    fn from_value(value: Value) -> VmResult<Self> {
        let stream = vm_try!(value.into_stream());
        VmResult::Ok(vm_try!(stream.take()))
    }
}

impl UnsafeFromValue for &Stream<Vm> {
    type Output = *const Stream<Vm>;
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let stream = vm_try!(value.into_stream());
        let (stream, guard) = Ref::into_raw(vm_try!(stream.into_ref()));
        VmResult::Ok((stream, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Stream<Vm> {
    type Output = *mut Stream<Vm>;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let stream = vm_try!(value.into_stream());
        VmResult::Ok(Mut::into_raw(vm_try!(stream.into_mut())))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
