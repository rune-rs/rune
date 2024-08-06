use core::fmt;

use crate::alloc::clone::TryClone;
use crate::runtime::{GeneratorState, Mut, Value, Vm, VmErrorKind, VmExecution, VmResult};
use crate::Any;

/// A stream with a stored virtual machine.
#[derive(Any)]
#[rune(crate)]
#[rune(builtin, static_type = STREAM, from_value_params = [Vm])]
#[rune(from_value = Value::into_stream, from_value_ref = Value::into_stream_ref, from_value_mut = Value::into_stream_mut)]
pub struct Stream<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    execution: Option<VmExecution<T>>,
}

impl<T> Stream<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
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
        let Some(execution) = self.execution.as_mut() else {
            return VmResult::Ok(None);
        };

        let state = if execution.is_resumed() {
            vm_try!(execution.async_resume_with(Value::empty()).await)
        } else {
            vm_try!(execution.async_resume().await)
        };

        VmResult::Ok(match state {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => {
                self.execution = None;
                None
            }
        })
    }

    pub(crate) async fn next_shared(mut this: Mut<Stream<T>>) -> VmResult<Option<Value>> {
        this.next().await
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

    pub(crate) async fn resume_shared(
        mut this: Mut<Stream<T>>,
        value: Value,
    ) -> VmResult<GeneratorState> {
        this.resume(value).await
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

impl<T> fmt::Debug for Stream<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl<T> TryClone for Stream<T>
where
    T: TryClone + AsRef<Vm> + AsMut<Vm>,
{
    fn try_clone(&self) -> crate::alloc::Result<Self> {
        Ok(Self {
            execution: self.execution.try_clone()?,
        })
    }
}
