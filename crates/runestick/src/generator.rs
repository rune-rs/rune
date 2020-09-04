use crate::{
    FromValue, GeneratorState, OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, Shared,
    UnsafeFromValue, Value, Vm, VmError, VmErrorKind, VmExecution,
};
use std::fmt;
use std::mem;

value_types!(crate::GENERATOR_TYPE, Generator => Generator, &Generator, &mut Generator);

/// A generator with a stored virtual machine.
pub struct Generator {
    execution: Option<VmExecution>,
    first: bool,
}

impl Generator {
    /// Construct a generator from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self {
            execution: Some(VmExecution::of(vm)),
            first: true,
        }
    }

    /// Get the next value produced by this stream.
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        Ok(match self.resume(Value::Unit)? {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => None,
        })
    }

    /// Get the next value produced by this stream.
    pub fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = match &mut self.execution {
            Some(execution) => execution,
            None => {
                return Err(VmError::from(VmErrorKind::GeneratorComplete));
            }
        };

        if !mem::take(&mut self.first) {
            execution.vm_mut()?.stack_mut().push(value);
        }

        let state = execution.resume()?;

        if state.is_complete() {
            self.execution = None;
        }

        Ok(state)
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl FromValue for Shared<Generator> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_generator()?)
    }
}

impl FromValue for Generator {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let generator = value.into_generator()?;
        Ok(generator.take()?)
    }
}

impl UnsafeFromValue for &Generator {
    type Output = *const Generator;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_generator()?;
        let (generator, guard) = OwnedRef::into_raw(generator.owned_ref()?);
        Ok((generator, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Generator {
    type Output = *mut Generator;
    type Guard = RawOwnedMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_generator()?;
        Ok(OwnedMut::into_raw(generator.owned_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}
