use crate::{
    vm::StopReason, FromValue, GeneratorState, OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef,
    Shared, UnsafeFromValue, Value, ValueError, Vm, VmError, VmErrorKind,
};
use std::fmt;
use std::mem;

value_types!(crate::GENERATOR_TYPE, Generator => Generator, &Generator, &mut Generator);

/// A generator with a stored virtual machine.
pub struct Generator {
    vm: Option<Vm>,
    first: bool,
}

impl Generator {
    /// Construct a generator from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self {
            vm: Some(vm),
            first: true,
        }
    }

    /// Get the next value produced by this generator.
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        let vm = match &mut self.vm {
            Some(vm) => vm,
            None => {
                return Err(VmError::from(VmErrorKind::GeneratorComplete));
            }
        };

        if !mem::take(&mut self.first) {
            vm.stack_mut().push(Value::Unit);
        }

        match Self::inner_resume(vm) {
            Ok(GeneratorState::Yielded(value)) => Ok(Some(value)),
            Ok(GeneratorState::Complete(_)) => {
                self.vm = None;
                Ok(None)
            }
            Err(error) => Err(error.into_unwinded(vm.ip())),
        }
    }

    /// Get the next value produced by this generator.
    pub fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let vm = match &mut self.vm {
            Some(vm) => vm,
            None => {
                return Err(VmError::from(VmErrorKind::GeneratorComplete));
            }
        };

        if !mem::take(&mut self.first) {
            vm.stack_mut().push(value);
        }

        match Self::inner_resume(vm) {
            Ok(value) => {
                if value.is_complete() {
                    self.vm = None;
                }

                Ok(value)
            }
            Err(error) => Err(error.into_unwinded(vm.ip())),
        }
    }

    /// Inner resume implementation.
    #[inline]
    fn inner_resume(vm: &mut Vm) -> Result<GeneratorState, VmError> {
        let reason = vm.run_for(None)?;

        match reason {
            StopReason::Yielded => Ok(GeneratorState::Yielded(vm.stack_mut().pop()?)),
            StopReason::Exited => Ok(GeneratorState::Complete(vm.stack_mut().pop()?)),
            reason => Err(VmError::from(VmErrorKind::Stopped {
                reason: reason.into_info(),
            })),
        }
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("completed", &self.vm.is_none())
            .finish()
    }
}

impl FromValue for Shared<Generator> {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_generator()?)
    }
}

impl FromValue for Generator {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        let generator = value.into_generator()?;
        Ok(generator.take()?)
    }
}

impl UnsafeFromValue for &Generator {
    type Output = *const Generator;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
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

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let generator = value.into_generator()?;
        Ok(OwnedMut::into_raw(generator.owned_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}
