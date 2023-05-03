use core::fmt;
use core::iter;

use crate::compile::Named;
use crate::runtime::{
    FromValue, GeneratorState, Iterator, Mut, RawMut, RawRef, RawStr, Ref, Shared, UnsafeFromValue,
    Value, Vm, VmErrorKind, VmExecution, VmResult,
};
use crate::InstallWith;

/// A generator with a stored virtual machine.
pub struct Generator<T>
where
    T: AsMut<Vm>,
{
    execution: Option<VmExecution<T>>,
}

impl<T> Generator<T>
where
    T: AsMut<Vm>,
{
    /// Construct a generator from a virtual machine.
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
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> VmResult<Option<Value>> {
        VmResult::Ok(match vm_try!(self.resume(Value::Unit)) {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => None,
        })
    }

    /// Resume the generator with a value and get the next generator state.
    pub fn resume(&mut self, value: Value) -> VmResult<GeneratorState> {
        let execution = vm_try!(self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete));

        let state = if execution.is_resumed() {
            vm_try!(execution.resume_with(value))
        } else {
            vm_try!(execution.resume())
        };

        if state.is_complete() {
            self.execution = None;
        }

        VmResult::Ok(state)
    }
}

impl Generator<&mut Vm> {
    /// Convert the current generator into one which owns its virtual machine.
    pub fn into_owned(self) -> Generator<Vm> {
        Generator {
            execution: self.execution.map(|e| e.into_owned()),
        }
    }
}

impl Generator<Vm> {
    /// Convert into iterator
    pub fn into_iterator(self) -> Iterator {
        Iterator::from("std::generator::GeneratorIterator", self.into_iter())
    }
}

impl IntoIterator for Generator<Vm> {
    type Item = VmResult<Value>;
    type IntoIter = GeneratorIterator;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        GeneratorIterator { generator: self }
    }
}

pub struct GeneratorIterator {
    generator: Generator<Vm>,
}

impl iter::Iterator for GeneratorIterator {
    type Item = VmResult<Value>;

    #[inline]
    fn next(&mut self) -> Option<VmResult<Value>> {
        match self.generator.next() {
            VmResult::Ok(Some(value)) => Some(VmResult::Ok(value)),
            VmResult::Ok(None) => None,
            VmResult::Err(error) => Some(VmResult::Err(error)),
        }
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
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_generator()
    }
}

impl FromValue for Generator<Vm> {
    fn from_value(value: Value) -> VmResult<Self> {
        let generator = vm_try!(value.into_generator());
        let generator = vm_try!(generator.take());
        VmResult::Ok(generator)
    }
}

impl UnsafeFromValue for &Generator<Vm> {
    type Output = *const Generator<Vm>;
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let generator = vm_try!(value.into_generator());
        let (generator, guard) = Ref::into_raw(vm_try!(generator.into_ref()));
        VmResult::Ok((generator, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Generator<Vm> {
    type Output = *mut Generator<Vm>;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let generator = vm_try!(value.into_generator());
        let generator = vm_try!(generator.into_mut());
        VmResult::Ok(Mut::into_raw(generator))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
