use crate::compile::Named;
use crate::runtime::{
    Call, FromValue, GeneratorState, Iterator, Mut, RawMut, RawRef, RawStr, Ref, Shared,
    UnsafeFromValue, Value, Vm, VmError, VmErrorKind, VmExecution,
};
use crate::InstallWith;
use std::fmt;
use std::mem;

/// A generator with a stored virtual machine.
pub struct Generator<T>
where
    T: AsMut<Vm>,
{
    execution: Option<VmExecution<T>>,
    first: bool,
}

impl<T> Generator<T>
where
    T: AsMut<Vm>,
{
    /// Construct a generator from a virtual machine.
    pub(crate) fn new(vm: T) -> Self {
        Self {
            execution: Some(VmExecution::new(vm, Call::Generator)),
            first: true,
        }
    }

    /// Construct a generator from a complete execution.
    pub(crate) fn from_execution(execution: VmExecution<T>) -> Self {
        let first = match execution.call {
            Call::Generator => true,
            _ => false,
        };

        Self {
            execution: Some(execution),
            first,
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

    /// Resume the generator with a value and get the next generator state.
    pub fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete)?;

        let state = if !mem::take(&mut self.first) {
            execution.resume_with(value)?
        } else {
            execution.resume()?
        };

        if state.is_complete() {
            self.execution = None;
        }

        Ok(state)
    }
}

impl Generator<&mut Vm> {
    /// Convert the current generator into one which owns its virtual machine.
    pub fn into_owned(self) -> Generator<Vm> {
        Generator {
            execution: self.execution.map(|e| e.into_owned()),
            first: self.first,
        }
    }
}

impl Generator<Vm> {
    /// Convert into iterator
    pub fn into_iterator(self) -> Result<Iterator, VmError> {
        Ok(Iterator::from(
            "std::generator::GeneratorIterator",
            self.into_iter(),
        ))
    }
}

impl IntoIterator for Generator<Vm> {
    type Item = Result<Value, VmError>;
    type IntoIter = GeneratorIterator;

    fn into_iter(self) -> Self::IntoIter {
        GeneratorIterator { generator: self }
    }
}

pub struct GeneratorIterator {
    generator: Generator<Vm>,
}

impl std::iter::Iterator for GeneratorIterator {
    type Item = Result<Value, VmError>;

    fn next(&mut self) -> Option<Result<Value, VmError>> {
        self.generator.next().transpose()
    }
}

impl<T> fmt::Debug for Generator<T>
where
    T: AsMut<Vm>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl<T> Named for Generator<T>
where
    T: AsMut<Vm>,
{
    const BASE_NAME: RawStr = RawStr::from_str("Generator");
}

impl<T> InstallWith for Generator<T> where T: AsMut<Vm> {}

impl FromValue for Shared<Generator<Vm>> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_generator()
    }
}

impl FromValue for Generator<Vm> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let generator = value.into_generator()?;
        Ok(generator.take()?)
    }
}

impl UnsafeFromValue for &Generator<Vm> {
    type Output = *const Generator<Vm>;
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

impl UnsafeFromValue for &mut Generator<Vm> {
    type Output = *mut Generator<Vm>;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let generator = value.into_generator()?;
        Ok(Mut::into_raw(generator.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
