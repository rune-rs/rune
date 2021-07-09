use crate::{
    FromValue, GeneratorState, InstallWith, Mut, Named, RawMut, RawRef, RawStr, Ref, Shared,
    UnsafeFromValue, Value, Vm, VmError, VmErrorKind, VmExecution,
};
use std::fmt;
use std::mem;

/// A generator with a stored virtual machine.
pub struct Generator {
    execution: Option<VmExecution>,
    first: bool,
}

impl Generator {
    /// Construct a generator from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self {
            execution: Some(VmExecution::new(vm)),
            first: true,
        }
    }

    /// Get the next value produced by this stream.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        Ok(match self.resume(Value::Unit)? {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => None,
        })
    }

    /// Get the next value produced by this stream.
    pub fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete)?;

        if !mem::take(&mut self.first) {
            execution.vm_mut()?.stack_mut().push(value);
        }

        let state = execution.resume()?;

        if state.is_complete() {
            self.execution = None;
        }

        Ok(state)
    }

    /// Convert into iterator
    pub fn into_iterator(self) -> Result<crate::Iterator, VmError> {
        Ok(crate::Iterator::from(
            "std::generator::GeneratorIterator",
            self.into_iter(),
        ))
    }
}

impl IntoIterator for Generator {
    type Item = Result<Value, VmError>;
    type IntoIter = GeneratorIterator;

    fn into_iter(self) -> GeneratorIterator {
        GeneratorIterator { generator: self }
    }
}

pub struct GeneratorIterator {
    generator: Generator,
}

impl std::iter::Iterator for GeneratorIterator {
    type Item = Result<Value, VmError>;

    fn next(&mut self) -> Option<Result<Value, VmError>> {
        self.generator.next().transpose()
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl Named for Generator {
    const BASE_NAME: RawStr = RawStr::from_str("Generator");
}

impl InstallWith for Generator {}

impl FromValue for Shared<Generator> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_generator()
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
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_generator()?;
        let (generator, guard) = Ref::into_raw(generator.into_ref()?);
        Ok((generator, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Generator {
    type Output = *mut Generator;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_generator()?;
        Ok(Mut::into_raw(generator.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
